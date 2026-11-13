//! The job queue.
//!
//! Every operation runs as a job on its own worker thread. At most
//! `max_concurrent` jobs run at once; the rest wait as `queued`. Jobs report
//! progress through a callback (the Tauri layer turns it into events), can be
//! cancelled while queued or running, and always clean up their temporary
//! folder. A panic inside an operation fails that job, not the application.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::AppError;
use crate::filesystem::paths::display_name;
use crate::filesystem::temp::TempDir;
use crate::services::context::{CancelToken, JobContext, ProgressUpdate};
use crate::services::operations::{self, OperationType, PdfOperation, ProcessingResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_finished(self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    #[serde(rename = "type")]
    pub op_type: OperationType,
    pub title: String,
    pub input_names: Vec<String>,
    pub status: JobStatus,
    /// 0..1, or `None` while the amount of work is not known.
    pub progress: Option<f64>,
    pub message: String,
    pub current_file: Option<String>,
    pub result: Option<ProcessingResult>,
    pub error: Option<AppError>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
}

pub type Listener = Arc<dyn Fn(&JobSnapshot) + Send + Sync>;

struct Entry {
    snapshot: JobSnapshot,
    cancel: CancelToken,
}

struct Inner {
    jobs: Mutex<HashMap<String, Entry>>,
    running: Mutex<usize>,
    slot_freed: Condvar,
    max_concurrent: usize,
    temp_root: PathBuf,
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Inner>,
}

const KEEP_FINISHED: usize = 100;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl JobManager {
    pub fn new(temp_root: PathBuf, max_concurrent: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                jobs: Mutex::new(HashMap::new()),
                running: Mutex::new(0),
                slot_freed: Condvar::new(),
                max_concurrent: max_concurrent.max(1),
                temp_root,
            }),
        }
    }

    /// Queues a validated operation. `on_update` receives every state
    /// change; `on_finish` runs once when the job ends.
    pub fn submit(&self, op: PdfOperation, inputs: Vec<PathBuf>, on_update: Listener, on_finish: Listener) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let cancel = CancelToken::new();
        let snapshot = JobSnapshot {
            id: id.clone(),
            op_type: op.op_type,
            title: op.op_type.title().to_string(),
            input_names: inputs.iter().map(|p| display_name(p)).collect(),
            status: JobStatus::Queued,
            progress: Some(0.0),
            message: "Waiting to start".into(),
            current_file: None,
            result: None,
            error: None,
            created_at: now_ms(),
            finished_at: None,
        };
        {
            let mut jobs = self.inner.jobs.lock().unwrap();
            prune_finished(&mut jobs);
            jobs.insert(id.clone(), Entry { snapshot: snapshot.clone(), cancel: cancel.clone() });
        }
        on_update(&snapshot);

        let inner = self.inner.clone();
        let job_id = id.clone();
        std::thread::Builder::new()
            .name(format!("job-{}", &id[..8]))
            .spawn(move || run_job(inner, job_id, op, inputs, cancel, on_update, on_finish))
            .expect("spawn job thread");
        id
    }

    /// Requests cancellation. Returns false for unknown or finished jobs.
    pub fn cancel(&self, id: &str) -> bool {
        let jobs = self.inner.jobs.lock().unwrap();
        match jobs.get(id) {
            Some(e) if !e.snapshot.status.is_finished() => {
                e.cancel.cancel();
                true
            }
            _ => false,
        }
    }

    pub fn list(&self) -> Vec<JobSnapshot> {
        let jobs = self.inner.jobs.lock().unwrap();
        let mut list: Vec<_> = jobs.values().map(|e| e.snapshot.clone()).collect();
        list.sort_by_key(|s| s.created_at);
        list
    }

    pub fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.inner.jobs.lock().unwrap().get(id).map(|e| e.snapshot.clone())
    }

    pub fn has_active(&self) -> bool {
        self.inner.jobs.lock().unwrap().values().any(|e| !e.snapshot.status.is_finished())
    }

    /// Forgets finished jobs (the interface's "Clear" in the job panel).
    pub fn clear_finished(&self) {
        self.inner.jobs.lock().unwrap().retain(|_, e| !e.snapshot.status.is_finished());
    }
}

fn prune_finished(jobs: &mut HashMap<String, Entry>) {
    let mut finished: Vec<(i64, String)> =
        jobs.iter().filter(|(_, e)| e.snapshot.status.is_finished()).map(|(k, e)| (e.snapshot.created_at, k.clone())).collect();
    if finished.len() > KEEP_FINISHED {
        finished.sort();
        for (_, k) in finished.iter().take(finished.len() - KEEP_FINISHED) {
            jobs.remove(k);
        }
    }
}

fn update(inner: &Inner, id: &str, listener: &Listener, f: impl FnOnce(&mut JobSnapshot)) {
    let snap = {
        let mut jobs = inner.jobs.lock().unwrap();
        let Some(entry) = jobs.get_mut(id) else { return };
        f(&mut entry.snapshot);
        entry.snapshot.clone()
    };
    listener(&snap);
}

fn run_job(
    inner: Arc<Inner>,
    id: String,
    op: PdfOperation,
    inputs: Vec<PathBuf>,
    cancel: CancelToken,
    on_update: Listener,
    on_finish: Listener,
) {
    // Wait for a free slot, staying cancellable while queued.
    {
        let mut running = inner.running.lock().unwrap();
        while *running >= inner.max_concurrent {
            if cancel.is_cancelled() {
                break;
            }
            running = inner.slot_freed.wait_timeout(running, Duration::from_millis(100)).unwrap().0;
        }
        if cancel.is_cancelled() {
            drop(running);
            finish(&inner, &id, &on_update, &on_finish, Err(AppError::cancelled()));
            return;
        }
        *running += 1;
    }

    update(&inner, &id, &on_update, |s| {
        s.status = JobStatus::Running;
        s.message = "Starting".into();
    });

    // Progress events are throttled so a fast loop cannot flood the interface.
    let last_emit = Arc::new(Mutex::new((Instant::now() - Duration::from_secs(1), String::new())));
    let sink_inner = inner.clone();
    let sink_id = id.clone();
    let sink_listener = on_update.clone();
    let sink = Arc::new(move |u: ProgressUpdate| {
        {
            let mut last = last_emit.lock().unwrap();
            let changed_message = last.1 != u.message;
            if !changed_message && last.0.elapsed() < Duration::from_millis(80) {
                // Still record the latest state without emitting.
                if let Some(e) = sink_inner.jobs.lock().unwrap().get_mut(&sink_id) {
                    e.snapshot.progress = u.fraction;
                }
                return;
            }
            *last = (Instant::now(), u.message.clone());
        }
        update(&sink_inner, &sink_id, &sink_listener, |s| {
            s.progress = u.fraction;
            s.message = u.message;
            if u.current_file.is_some() {
                s.current_file = u.current_file;
            }
        });
    });

    let outcome = match TempDir::create(&inner.temp_root, &id) {
        Err(e) => Err(e),
        Ok(temp) => {
            let ctx = JobContext::new(cancel.clone(), sink, temp);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let staged = operations::execute(&ctx, &op, &inputs)?;
                ctx.check()?;
                operations::finalize(staged, &op, &inputs)
            }));
            // `ctx` (and with it the temporary folder) is dropped here.
            match result {
                Ok(r) => r,
                Err(_) => Err(AppError::processing("Something went wrong while processing this document.")
                    .with_hint("The document may use features PaperFlow does not support yet.")),
            }
        }
    };

    {
        let mut running = inner.running.lock().unwrap();
        *running -= 1;
    }
    inner.slot_freed.notify_all();
    finish(&inner, &id, &on_update, &on_finish, outcome);
}

fn finish(
    inner: &Inner,
    id: &str,
    on_update: &Listener,
    on_finish: &Listener,
    outcome: Result<ProcessingResult, AppError>,
) {
    update(inner, id, on_update, |s| {
        s.finished_at = Some(now_ms());
        match outcome {
            Ok(result) => {
                s.status = JobStatus::Completed;
                s.progress = Some(1.0);
                s.message = "Done".into();
                s.result = Some(result);
            }
            Err(e) if e.is_cancelled() => {
                s.status = JobStatus::Cancelled;
                s.message = "Cancelled".into();
                s.error = Some(e);
            }
            Err(e) => {
                s.status = JobStatus::Failed;
                s.message = e.message.clone();
                s.error = Some(e);
            }
        }
    });
    if let Some(snap) = inner.jobs.lock().unwrap().get(id).map(|e| e.snapshot.clone()) {
        on_finish(&snap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::assemble::tests::sample_doc;
    use serde_json::json;
    use std::sync::mpsc;

    fn wait_for(rx: &mpsc::Receiver<JobSnapshot>, id: &str) -> JobSnapshot {
        loop {
            let s = rx.recv_timeout(Duration::from_secs(20)).expect("job finished");
            if s.id == id {
                return s;
            }
        }
    }

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        sample_doc(3, "A").save(&pdf).unwrap();
        (dir, pdf)
    }

    fn rotate_op(pdf: &PathBuf, out: &std::path::Path) -> PdfOperation {
        serde_json::from_value(json!({
            "type": "rotate", "inputFiles": [pdf], "outputDir": out, "options": {"angle": 180}
        }))
        .unwrap()
    }

    #[test]
    fn runs_jobs_to_completion_and_cleans_up() {
        let (dir, pdf) = setup();
        let temp_root = dir.path().join("cache");
        let mgr = JobManager::new(temp_root.clone(), 2);
        let (tx, rx) = mpsc::channel();
        let updates = Arc::new(Mutex::new(Vec::new()));
        let u2 = updates.clone();
        let id = mgr.submit(
            rotate_op(&pdf, dir.path()),
            vec![pdf.clone()],
            Arc::new(move |s| u2.lock().unwrap().push(s.status)),
            Arc::new(move |s| tx.send(s.clone()).unwrap()),
        );
        let done = wait_for(&rx, &id);
        assert_eq!(done.status, JobStatus::Completed, "{:?}", done.error);
        let result = done.result.unwrap();
        assert!(PathBuf::from(&result.output_files[0]).exists());
        let seen = updates.lock().unwrap().clone();
        assert_eq!(seen.first(), Some(&JobStatus::Queued));
        assert!(seen.contains(&JobStatus::Running));
        assert!(!mgr.has_active());
        assert_eq!(crate::filesystem::temp::usage(&temp_root), 0, "temporary files are removed");
    }

    #[test]
    fn failures_are_reported_not_panicked() {
        let (dir, pdf) = setup();
        let mgr = JobManager::new(dir.path().join("cache"), 1);
        let (tx, rx) = mpsc::channel();
        let op: PdfOperation = serde_json::from_value(json!({
            "type": "extractPages", "inputFiles": [pdf], "outputDir": dir.path(), "options": {"pages": "99"}
        }))
        .unwrap();
        let id = mgr.submit(op, vec![pdf], Arc::new(|_| {}), Arc::new(move |s| tx.send(s.clone()).unwrap()));
        let done = wait_for(&rx, &id);
        assert_eq!(done.status, JobStatus::Failed);
        assert!(done.error.unwrap().message.contains("outside"));
    }

    #[test]
    fn queued_jobs_can_be_cancelled() {
        let (dir, pdf) = setup();
        let mgr = JobManager::new(dir.path().join("cache"), 1);
        // Occupy the only slot so the next job stays queued.
        *mgr.inner.running.lock().unwrap() = 1;
        let (tx, rx) = mpsc::channel();
        let id = mgr.submit(rotate_op(&pdf, dir.path()), vec![pdf], Arc::new(|_| {}), Arc::new(move |s| tx.send(s.clone()).unwrap()));
        assert_eq!(mgr.get(&id).unwrap().status, JobStatus::Queued);
        assert!(mgr.cancel(&id));
        let done = wait_for(&rx, &id);
        assert_eq!(done.status, JobStatus::Cancelled);
        assert!(!mgr.cancel(&id), "finished jobs cannot be cancelled again");
        mgr.clear_finished();
        assert!(mgr.list().is_empty());
    }
}
