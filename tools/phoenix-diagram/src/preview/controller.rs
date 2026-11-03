use super::{http::Server, PreviewFailure, PreviewRequest, PreviewResult};
use crate::{delivery::prepare, diagnostic::Diagnostics, input::sha256};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(800);
const STABLE: Duration = Duration::from_millis(400);
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn signal(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
struct Signals {
    interrupt: libc::sigaction,
    terminate: libc::sigaction,
}
impl Signals {
    fn install() -> PreviewResult<Self> {
        STOP.store(false, Ordering::Relaxed);
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = signal as *const () as usize;
            libc::sigemptyset(&mut action.sa_mask);
            let mut interrupt = std::mem::zeroed();
            let mut terminate = std::mem::zeroed();
            if libc::sigaction(libc::SIGINT, &action, &mut interrupt) != 0 {
                return Err(PreviewFailure::io(
                    "preview/signal",
                    io::Error::last_os_error(),
                ));
            }
            if libc::sigaction(libc::SIGTERM, &action, &mut terminate) != 0 {
                libc::sigaction(libc::SIGINT, &interrupt, std::ptr::null_mut());
                return Err(PreviewFailure::io(
                    "preview/signal",
                    io::Error::last_os_error(),
                ));
            }
            Ok(Self {
                interrupt,
                terminate,
            })
        }
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(libc::SIGINT, &self.interrupt, std::ptr::null_mut());
            libc::sigaction(libc::SIGTERM, &self.terminate, std::ptr::null_mut());
        }
    }
}

/// Input edits settle for STABLE before one in-process compile; a newer edit
/// replaces any compile still waiting.
struct Pending {
    generation: u64,
    ready_at: Instant,
    snapshot: Snapshot,
}
enum Snapshot {
    Source(Vec<u8>),
    Unavailable(PreviewFailure),
}
impl Snapshot {
    fn read(path: &Path) -> Self {
        match std::fs::read(path) {
            Ok(bytes) => Self::Source(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Self::Unavailable(PreviewFailure::new(
                    "input/missing",
                    "The preview input is missing; the last successful file is unchanged",
                    1,
                ))
            }
            Err(error) => Self::Unavailable(PreviewFailure::io("input/read", error)),
        }
    }
    fn stamp(&self) -> String {
        match self {
            Self::Source(bytes) => format!("sha256:{}", sha256(bytes)),
            Self::Unavailable(failure) => format!(
                "unavailable:{}",
                failure
                    .diagnostics
                    .first()
                    .map(|d| d.code.as_str())
                    .unwrap_or("input/unavailable")
            ),
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Artifact {
    generation: u64,
    input_sha256: String,
    sha256: String,
    bytes: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct State {
    schema_version: u32,
    tool: &'static str,
    tool_version: &'static str,
    command: &'static str,
    status: &'static str,
    session_id: String,
    generation: u64,
    last_successful_generation: u64,
    last_attempt_status: &'static str,
    diagnostics: Diagnostics,
    artifact: Option<Artifact>,
    artifact_available: bool,
    locale: String,
    browser_status: &'static str,
    visual_review_status: &'static str,
}
impl State {
    fn new(session_id: String) -> Self {
        Self {
            schema_version: 1,
            tool: "phoenix-diagram",
            tool_version: crate::TOOL_VERSION,
            command: "preview",
            status: "waiting",
            session_id,
            generation: 0,
            last_successful_generation: 0,
            last_attempt_status: "not-run",
            diagnostics: vec![],
            artifact: None,
            artifact_available: false,
            locale: "en".into(),
            browser_status: "not-run",
            visual_review_status: "not-run",
        }
    }
    fn event(&self, event: &str, extra: Value) -> io::Result<()> {
        let mut value = serde_json::to_value(self).map_err(io::Error::other)?;
        value["event"] = json!(event);
        for (key, value_extra) in extra.as_object().into_iter().flatten() {
            value[key] = value_extra.clone();
        }
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(&mut stdout, &value).map_err(io::Error::other)?;
        stdout.write_all(b"\n")?;
        stdout.flush()
    }
    fn failed(&mut self, failure: PreviewFailure) -> io::Result<()> {
        self.status = "error";
        self.last_attempt_status = "failed";
        self.diagnostics = failure.diagnostics;
        self.event("failed", json!({"exitCode":failure.exit}))
    }
    fn observe(&mut self, snapshot: &Snapshot, generation: u64) {
        self.generation = generation;
        self.status = "waiting";
        self.last_attempt_status = "not-run";
        self.diagnostics.clear();
        if let Snapshot::Source(bytes) = snapshot {
            if let Ok(document) = serde_json::from_slice::<Value>(bytes) {
                self.locale = crate::i18n::resolve(&document["meta"]).resolved;
            }
        }
    }
}
fn emit_failure(state: &mut State, failure: PreviewFailure) -> PreviewResult<()> {
    state
        .failed(failure)
        .map_err(|e| PreviewFailure::io("preview/events", e))
}
/// Compile one settled input and, when it passes, write the output file.
fn compile(request: &PreviewRequest<'_>, bytes: &[u8]) -> PreviewResult<Vec<u8>> {
    let prepared = prepare::prepare_bytes(&request.compile, bytes);
    let Some(compiled) = prepared.compiled else {
        let exit = prepared.receipt.exit_code().max(1);
        return Err(PreviewFailure {
            diagnostics: if prepared.receipt.diagnostics.is_empty() {
                PreviewFailure::new("preview/compile", "Preview compilation did not pass", exit)
                    .diagnostics
            } else {
                prepared.receipt.diagnostics
            },
            exit,
        });
    };
    crate::output::write(request.output, &compiled.html)
        .map_err(|e| PreviewFailure::io("output/write", e))?;
    Ok(compiled.html)
}
fn session_token() -> PreviewResult<String> {
    let mut random = [0_u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(|e| PreviewFailure::io("preview/session", e))?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn open_browser(url: &str) -> PreviewResult<()> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(target_os = "macos"))]
    let program = "xdg-open";
    let mut child = Command::new(program)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| PreviewFailure::io("preview/open", e))?;
    let _ = std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub(super) fn run(request: &PreviewRequest<'_>) -> u8 {
    let mut state = State::new(String::new());
    let result = session_token().and_then(|session| {
        state.session_id = session.clone();
        run_session(request, &session, &mut state)
    });
    match result {
        Ok(exit) => exit,
        Err(failure) => {
            let exit = failure.exit;
            let _ = state.failed(failure);
            exit
        }
    }
}
fn run_session(
    request: &PreviewRequest<'_>,
    session: &str,
    state: &mut State,
) -> PreviewResult<u8> {
    let _signals = Signals::install()?;
    if let (Ok(input), Ok(output)) = (
        std::fs::canonicalize(request.compile.candidate),
        std::fs::canonicalize(request.output),
    ) {
        if input == output {
            return Err(PreviewFailure::new(
                "preview/input-output-alias",
                "Preview input and output must be different files",
                2,
            ));
        }
    }
    let mut server = Server::bind(request.port, session)?;
    let started = state.event("started", json!({"url":server.url(),"port":server.port()}));
    if let Err(error) = started {
        server.stop();
        return Err(PreviewFailure::io("preview/events", error));
    }
    if request.open {
        if let Err(failure) = open_browser(&server.url()) {
            emit_failure(state, failure)?;
        }
    }
    let mut generation = 0;
    let mut stamp = None;
    let mut pending: Option<Pending> = None;
    let mut published: Option<Arc<[u8]>> = None;
    let mut last_poll: Option<Instant> = None;
    let mut exit = 0;
    let work = (|| -> PreviewResult<()> {
        while !STOP.load(Ordering::Relaxed) {
            if last_poll.is_none_or(|time| time.elapsed() >= POLL) {
                let snapshot = Snapshot::read(request.compile.candidate);
                let current = snapshot.stamp();
                if stamp.as_ref() != Some(&current) {
                    generation += 1;
                    stamp = Some(current);
                    state.observe(&snapshot, generation);
                    pending = Some(Pending {
                        generation,
                        ready_at: Instant::now() + STABLE,
                        snapshot,
                    });
                }
                last_poll = Some(Instant::now());
            }
            if pending
                .as_ref()
                .is_some_and(|pending| pending.ready_at <= Instant::now())
            {
                let job = pending.take().expect("ready pending input");
                match job.snapshot {
                    Snapshot::Unavailable(failure) => emit_failure(state, failure)?,
                    Snapshot::Source(bytes) => {
                        state.status = "checking";
                        state.last_attempt_status = "checking";
                        state
                            .event("checking", json!({}))
                            .map_err(|e| PreviewFailure::io("preview/events", e))?;
                        match compile(request, &bytes) {
                            Err(failure) => emit_failure(state, failure)?,
                            Ok(html) => {
                                state.artifact = Some(Artifact {
                                    generation: job.generation,
                                    input_sha256: sha256(&bytes),
                                    sha256: sha256(&html),
                                    bytes: html.len() as u64,
                                });
                                published = Some(Arc::from(html));
                                state.status = "ready";
                                state.last_attempt_status = "pass";
                                state.last_successful_generation = job.generation;
                                state.artifact_available = true;
                                state.diagnostics.clear();
                                state
                                    .event("updated", json!({}))
                                    .map_err(|e| PreviewFailure::io("preview/events", e))?;
                            }
                        }
                    }
                }
            }
            let status = serde_json::to_vec(state)
                .map_err(|error| PreviewFailure::new("preview/status", error.to_string(), 4))?;
            server.poll(&status, published.as_ref())?;
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    })();
    if let Err(failure) = work {
        exit = failure.exit;
        let _ = state.failed(failure);
    }
    let shutdown_complete = server.stop();
    if !shutdown_complete {
        exit = exit.max(4);
    }
    state.status = "stopped";
    state
        .event(
            "stopped",
            json!({"shutdownComplete":shutdown_complete,"exitCode":exit}),
        )
        .map_err(|e| PreviewFailure::io("preview/events", e))?;
    Ok(exit)
}
