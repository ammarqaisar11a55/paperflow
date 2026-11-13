//! Starting, cancelling and listing jobs.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::error::AppResult;
use crate::services::jobs::JobSnapshot;
use crate::services::operations::{self, PdfOperation};
use crate::services::store::ActivityEntry;
use crate::AppState;

pub const JOB_EVENT: &str = "job-update";

/// Validates and queues an operation. Returns the job id; progress and the
/// result arrive as `job-update` events.
#[tauri::command]
pub fn start_job<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    operation: PdfOperation,
) -> AppResult<String> {
    let inputs = operations::validate(&operation)?;
    let emitter = app.clone();
    let store = state.store.clone();
    let id = state.jobs.submit(
        operation,
        inputs,
        Arc::new(move |snapshot: &JobSnapshot| {
            let _ = emitter.emit(JOB_EVENT, snapshot);
        }),
        Arc::new(move |snapshot: &JobSnapshot| {
            if let Err(e) = store.record(ActivityEntry::from_job(snapshot)) {
                log::warn!("could not record activity: {e}");
            }
        }),
    );
    Ok(id)
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: String) -> bool {
    state.jobs.cancel(&id)
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Vec<JobSnapshot> {
    state.jobs.list()
}

#[tauri::command]
pub fn clear_finished_jobs(state: State<'_, AppState>) {
    state.jobs.clear_finished()
}
