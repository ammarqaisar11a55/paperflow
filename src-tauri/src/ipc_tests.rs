//! End-to-end tests of the IPC contract: the real command handlers, invoked
//! on Tauri's mock runtime with exactly the JSON the interface sends
//! (see src/services/api.ts). They catch renamed arguments, changed field
//! names and serialization mismatches that unit tests on either side miss.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindow, WebviewWindowBuilder};

use crate::pdf::assemble::tests::sample_doc;
use crate::services::jobs::JobManager;
use crate::services::store::Store;
use crate::AppState;

struct Harness {
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    dir: tempfile::TempDir,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let temp_root = dir.path().join("jobs-root");
    let app = crate::with_commands(mock_builder())
        .manage(AppState {
            jobs: JobManager::new(temp_root.clone(), 2),
            store: Arc::new(Store::open(&dir.path().join("config"), &dir.path().join("data"))),
            temp_root,
        })
        .build(mock_context(noop_assets()))
        .expect("app builds");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default()).build().expect("window");
    Harness { _app: app, window, dir }
}

fn invoke(h: &Harness, cmd: &str, args: Value) -> Result<Value, Value> {
    let request = InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        // The app's own origin on Linux; anything else counts as remote content.
        url: "tauri://localhost".parse().unwrap(),
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(&h.window, request).map(|body| match body {
        InvokeResponseBody::Json(s) => serde_json::from_str(&s).unwrap_or(Value::String(s)),
        InvokeResponseBody::Raw(bytes) => json!({ "rawLength": bytes.len() }),
    })
}

#[test]
fn inspects_files_and_reads_bytes() {
    let h = harness();
    let pdf = h.dir.path().join("a.pdf");
    sample_doc(3, "A").save(&pdf).unwrap();
    let txt = h.dir.path().join("b.txt");
    std::fs::write(&txt, b"hello").unwrap();

    let infos = invoke(&h, "inspect_files", json!({ "paths": [pdf, txt] })).unwrap();
    assert_eq!(infos[0]["kind"], "pdf");
    assert_eq!(infos[0]["pageCount"], 3);
    assert_eq!(infos[0]["encrypted"], false);
    assert_eq!(infos[1]["kind"], "unsupported");
    assert!(infos[1]["error"].is_string());

    let bytes = invoke(&h, "read_file_bytes", json!({ "path": pdf })).unwrap();
    assert_eq!(bytes["rawLength"].as_u64().unwrap(), std::fs::metadata(&pdf).unwrap().len());

    let err = invoke(&h, "read_file_bytes", json!({ "path": txt })).unwrap_err();
    assert_eq!(err["kind"], "invalidInput");
    assert!(err["message"].as_str().unwrap().contains("not a PDF"));
}

#[test]
fn settings_round_trip_with_interface_field_names() {
    let h = harness();
    let mut settings = invoke(&h, "get_settings", json!({})).unwrap();
    assert_eq!(settings["theme"], "system");
    assert_eq!(settings["outputLocation"], "sameAsSource");
    settings["theme"] = json!("dark");
    settings["favorites"] = json!(["compress"]);
    let saved = invoke(&h, "save_settings", json!({ "settings": settings })).unwrap();
    assert_eq!(saved["theme"], "dark");
    assert_eq!(invoke(&h, "get_settings", json!({})).unwrap()["favorites"], json!(["compress"]));
}

#[test]
fn starts_a_job_and_records_activity() {
    let h = harness();
    let pdf = h.dir.path().join("report.pdf");
    sample_doc(2, "A").save(&pdf).unwrap();
    let operation = json!({
        "type": "rotate",
        "inputFiles": [pdf],
        "outputDir": h.dir.path(),
        "outputName": "report - rotated",
        "options": { "angle": 90, "pages": "" },
        "overwrite": "rename",
    });
    let id = invoke(&h, "start_job", json!({ "operation": operation })).unwrap();
    let id = id.as_str().unwrap().to_string();

    let deadline = Instant::now() + Duration::from_secs(20);
    let job = loop {
        let jobs = invoke(&h, "list_jobs", json!({})).unwrap();
        let job = jobs.as_array().unwrap().iter().find(|j| j["id"] == id.as_str()).cloned().unwrap();
        if ["completed", "failed", "cancelled"].contains(&job["status"].as_str().unwrap()) {
            break job;
        }
        assert!(Instant::now() < deadline, "job did not finish");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(job["status"], "completed", "{job}");
    assert_eq!(job["type"], "rotate");
    assert_eq!(job["inputNames"], json!(["report.pdf"]));
    let output = job["result"]["outputFiles"][0].as_str().unwrap();
    assert!(output.ends_with("report - rotated.pdf"));
    assert!(std::path::Path::new(output).exists());
    assert_eq!(job["result"]["pageCount"], 2);

    let activity = invoke(&h, "get_activity", json!({})).unwrap();
    assert_eq!(activity[0]["id"], id.as_str());
    assert_eq!(activity[0]["fileNames"], json!(["report.pdf"]));
    assert_eq!(activity[0]["status"], "completed");
}

#[test]
fn validation_errors_reach_the_interface_before_queueing() {
    let h = harness();
    let operation = json!({
        "type": "merge",
        "inputFiles": [h.dir.path().join("missing.pdf")],
        "outputDir": h.dir.path(),
        "options": {},
    });
    let err = invoke(&h, "start_job", json!({ "operation": operation })).unwrap_err();
    assert_eq!(err["kind"], "invalidInput");
    assert!(err["hint"].is_string());
    assert_eq!(invoke(&h, "list_jobs", json!({})).unwrap(), json!([]));
}

#[test]
fn reports_engines_and_app_info() {
    let h = harness();
    let engines = invoke(&h, "engine_status", json!({})).unwrap();
    let names: Vec<&str> =
        engines.as_array().unwrap().iter().map(|e| e["binary"].as_str().unwrap()).collect();
    assert_eq!(names, ["gs", "qpdf", "pdftoppm", "pdftotext", "pdfimages", "tesseract"]);
    let info = invoke(&h, "app_info", json!({})).unwrap();
    assert_eq!(info["license"], "MIT");
    assert_eq!(invoke(&h, "temp_usage", json!({})).unwrap(), 0);
    let dir = invoke(&h, "suggest_output_dir", json!({ "input": h.dir.path().join("x.pdf") })).unwrap();
    assert_eq!(dir.as_str().unwrap(), h.dir.path().display().to_string());
    let _ = h.window.app_handle();
}
