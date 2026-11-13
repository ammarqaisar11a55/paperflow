//! Locating and running the bundled processing engines.
//!
//! PaperFlow does structural PDF work (merge, split, rotate, organize,
//! watermark, page numbers, metadata, images to PDF) in Rust. It calls out to
//! established open-source command-line engines for the rest:
//!
//! | Engine      | Used for                                   |
//! |-------------|--------------------------------------------|
//! | Ghostscript | Compression                                |
//! | Poppler     | Rendering to images, text, image extraction|
//! | qpdf        | Password protection and unlocking          |
//! | Tesseract   | OCR                                        |
//!
//! Inside the Snap they live in `$SNAP/usr/bin`. During development they come
//! from `PATH`, or from `PAPERFLOW_ENGINE_DIR` (see `scripts/dev-engines.sh`).
//! Engines never receive document contents over the command line, and
//! passwords go through stdin, never through arguments.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorKind};
use crate::services::context::CancelToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Engine {
    Ghostscript,
    Qpdf,
    Pdftoppm,
    Pdftotext,
    Pdfimages,
    Tesseract,
}

impl Engine {
    pub const ALL: [Engine; 6] = [
        Engine::Ghostscript,
        Engine::Qpdf,
        Engine::Pdftoppm,
        Engine::Pdftotext,
        Engine::Pdfimages,
        Engine::Tesseract,
    ];

    pub fn binary(self) -> &'static str {
        match self {
            Engine::Ghostscript => "gs",
            Engine::Qpdf => "qpdf",
            Engine::Pdftoppm => "pdftoppm",
            Engine::Pdftotext => "pdftotext",
            Engine::Pdfimages => "pdfimages",
            Engine::Tesseract => "tesseract",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Engine::Ghostscript => "Ghostscript",
            Engine::Qpdf => "qpdf",
            Engine::Pdftoppm | Engine::Pdftotext | Engine::Pdfimages => "Poppler",
            Engine::Tesseract => "Tesseract OCR",
        }
    }

    fn package(self) -> &'static str {
        match self {
            Engine::Ghostscript => "ghostscript",
            Engine::Qpdf => "qpdf",
            Engine::Pdftoppm | Engine::Pdftotext | Engine::Pdfimages => "poppler-utils",
            Engine::Tesseract => "tesseract-ocr",
        }
    }

    fn version_args(self) -> &'static [&'static str] {
        match self {
            Engine::Ghostscript => &["--version"],
            Engine::Qpdf | Engine::Tesseract => &["--version"],
            Engine::Pdftoppm | Engine::Pdftotext | Engine::Pdfimages => &["-v"],
        }
    }
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(v) = std::env::var_os("PAPERFLOW_ENGINE_DIR") {
        dirs.extend(std::env::split_paths(&v));
    }
    if let Some(snap) = std::env::var_os("SNAP") {
        let snap = PathBuf::from(snap);
        dirs.push(snap.join("usr/bin"));
        dirs.push(snap.join("bin"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs
}

/// Finds an engine executable, or `None` when it is not installed.
pub fn locate(engine: Engine) -> Option<PathBuf> {
    search_dirs().into_iter().map(|d| d.join(engine.binary())).find(|p| is_executable(p))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// The folder holding Tesseract's language data, if one can be found.
pub fn tessdata_dir() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("TESSDATA_PREFIX") {
        let p = PathBuf::from(v);
        if p.is_dir() {
            return Some(p);
        }
    }
    let mut roots = Vec::new();
    if let Some(snap) = std::env::var_os("SNAP") {
        roots.push(PathBuf::from(snap).join("usr/share/tesseract-ocr"));
    }
    if let Some(v) = std::env::var_os("PAPERFLOW_ENGINE_DIR") {
        for d in std::env::split_paths(&v) {
            if let Some(prefix) = d.parent() {
                roots.push(prefix.join("share/tesseract-ocr"));
            }
        }
    }
    roots.push(PathBuf::from("/usr/share/tesseract-ocr"));
    for root in roots {
        for version in ["5", "4.00", "4"] {
            let p = root.join(version).join("tessdata");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

/// Ghostscript looks for its startup files and fonts at the path it was
/// built for (/usr/share/ghostscript/<version>). Inside the Snap they live
/// under $SNAP, so point it there.
fn ghostscript_lib_path() -> Option<std::ffi::OsString> {
    let snap = PathBuf::from(std::env::var_os("SNAP")?);
    let share = snap.join("usr/share/ghostscript");
    let version_dir = std::fs::read_dir(&share)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("Resource/Init").is_dir())
        .max()?;
    let dirs = [
        version_dir.join("Resource/Init"),
        version_dir.join("lib"),
        version_dir.join("Resource/Font"),
        share.join("fonts"),
        snap.join("usr/share/fonts/type1/urw-base35"),
    ];
    std::env::join_paths(dirs.iter().filter(|d| d.is_dir())).ok()
}

/// Builds a command for an engine with a clean, predictable environment.
pub fn command(engine: Engine) -> AppResult<Command> {
    let path = locate(engine).ok_or_else(|| missing(engine))?;
    let mut cmd = Command::new(path);
    if let Some(libs) = std::env::var_os("PAPERFLOW_ENGINE_LIB") {
        let mut paths = std::env::split_paths(&libs).collect::<Vec<_>>();
        if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env("LD_LIBRARY_PATH", joined);
        }
    }
    if engine == Engine::Ghostscript {
        if let Some(lib) = ghostscript_lib_path() {
            cmd.env("GS_LIB", lib);
        }
    }
    if engine == Engine::Tesseract {
        if let Some(dir) = tessdata_dir() {
            cmd.env("TESSDATA_PREFIX", dir);
        }
        // One thread per page keeps the machine responsive; pages run in sequence.
        cmd.env("OMP_THREAD_LIMIT", "2");
    }
    // Messages must be parseable regardless of the user's locale.
    cmd.env("LC_ALL", "C");
    Ok(cmd)
}

pub fn missing(engine: Engine) -> AppError {
    AppError::new(ErrorKind::MissingEngine, format!("{} is not available.", engine.label())).with_hint(
        format!(
        "PaperFlow needs {} for this tool. The Snap package includes it; otherwise install the “{}” package.",
        engine.label(),
        engine.package()
    ),
    )
}

/// Version string of an installed engine, for Settings › About.
pub fn version(engine: Engine) -> Option<String> {
    let mut cmd = command(engine).ok()?;
    let out = cmd.args(engine.version_args()).stdin(Stdio::null()).output().ok()?;
    let text = if out.stdout.is_empty() { out.stderr } else { out.stdout };
    String::from_utf8_lossy(&text).lines().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string())
}

#[derive(Debug)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn success(&self) -> bool {
        self.status.success()
    }

    /// The last few lines of stderr, for an error's technical detail.
    pub fn stderr_tail(&self) -> String {
        let lines: Vec<&str> = self.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        lines[lines.len().saturating_sub(6)..].join("\n")
    }
}

/// Runs a command to completion while honouring cancellation. `on_line` is
/// called with every stdout line as it arrives (used for progress). `stdin`
/// is written to the child's standard input and then closed.
pub fn run<I, S>(
    mut cmd: Command,
    args: I,
    stdin: Option<&[u8]>,
    cancel: &CancelToken,
    mut on_line: impl FnMut(&str),
) -> AppResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    cancel.check()?;
    cmd.args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        AppError::processing("A processing engine could not be started.").with_detail(e.to_string())
    })?;

    if let Some(data) = stdin {
        if let Some(mut pipe) = child.stdin.take() {
            // Small payloads (argument files); write before reading output.
            let _ = pipe.write_all(data);
        }
    }

    let stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let (tx, rx) = mpsc::channel::<String>();
    let out_reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    });

    let mut collected = String::new();
    let status = loop {
        match rx.recv_timeout(Duration::from_millis(60)) {
            Ok(line) => {
                on_line(&line);
                collected.push_str(&line);
                collected.push('\n');
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err(AppError::cancelled());
        }
        if let Some(status) = child.try_wait()? {
            // Drain anything still buffered.
            let _ = out_reader.join();
            for line in rx.try_iter() {
                on_line(&line);
                collected.push_str(&line);
                collected.push('\n');
            }
            break status;
        }
    };
    let stderr = err_reader.join().unwrap_or_default();
    Ok(Output { status, stdout: collected, stderr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_engine_error_names_the_package() {
        let e = missing(Engine::Qpdf);
        assert_eq!(e.kind, ErrorKind::MissingEngine);
        assert!(e.hint.unwrap().contains("qpdf"));
    }

    #[cfg(unix)]
    #[test]
    fn runs_and_streams_lines() {
        let cancel = CancelToken::new();
        let mut lines = Vec::new();
        let out = run(Command::new("sh"), ["-c", "echo one; echo two; echo err >&2"], None, &cancel, |l| {
            lines.push(l.to_string())
        })
        .unwrap();
        assert!(out.success());
        assert_eq!(lines, vec!["one", "two"]);
        assert_eq!(out.stderr.trim(), "err");
    }

    #[cfg(unix)]
    #[test]
    fn passes_stdin() {
        let cancel = CancelToken::new();
        let out = run(Command::new("cat"), Vec::<&str>::new(), Some(b"secret\n"), &cancel, |_| {}).unwrap();
        assert_eq!(out.stdout, "secret\n");
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_kills_the_child() {
        let cancel = CancelToken::new();
        let c2 = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            c2.cancel();
        });
        let started = std::time::Instant::now();
        let err = run(Command::new("sleep"), ["10"], None, &cancel, |_| {}).unwrap_err();
        assert!(err.is_cancelled());
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
