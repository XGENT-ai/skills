use super::{http::Server, worker, PreviewFailure, PreviewRequest, PreviewResult};
use crate::{
    check::FileIdentity,
    delivery::Status,
    diagnostic::Diagnostics,
    input::sha256,
    output::{self, Directory, FileBinding, OutputSlot},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    io::{self, Write},
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

struct Pending<T> {
    generation: u64,
    ready_at: Instant,
    value: T,
}
struct Latest<T> {
    generation: u64,
    stamp: Option<String>,
    pending: Option<Pending<T>>,
    active: Option<u64>,
}
impl<T> Latest<T> {
    fn new() -> Self {
        Self {
            generation: 0,
            stamp: None,
            pending: None,
            active: None,
        }
    }
    fn observe(&mut self, stamp: String, value: T, now: Instant) -> bool {
        if self.stamp.as_ref() == Some(&stamp) {
            return false;
        }
        self.generation += 1;
        self.stamp = Some(stamp);
        self.pending = Some(Pending {
            generation: self.generation,
            ready_at: now + STABLE,
            value,
        });
        true
    }
    fn replace_pending(&mut self, value: T, now: Instant) {
        self.pending = Some(Pending {
            generation: self.generation,
            ready_at: now + STABLE,
            value,
        });
    }
    fn start(&mut self, now: Instant) -> Option<Pending<T>> {
        if self.active.is_some()
            || self
                .pending
                .as_ref()
                .is_none_or(|pending| pending.ready_at > now)
        {
            return None;
        }
        let pending = self.pending.take().unwrap();
        self.active = Some(pending.generation);
        Some(pending)
    }
    fn finish(&mut self, generation: u64) -> bool {
        if self.active != Some(generation) {
            return false;
        }
        self.active = None;
        generation == self.generation
    }
}
enum Snapshot {
    Source(FileBinding),
    Unavailable(PreviewFailure),
}
impl Snapshot {
    fn stamp(&self) -> String {
        match self {
            Self::Source(binding) => format!("sha256:{}", binding.identity().sha256),
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
    fn source(&self) -> Option<&FileBinding> {
        match self {
            Self::Source(source) => Some(source),
            _ => None,
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
        if let Some(source) = snapshot.source() {
            if let Ok(document) = serde_json::from_slice::<Value>(source.bytes()) {
                self.locale = crate::i18n::resolve(&document["meta"]).resolved;
            }
        }
    }
}
struct Active {
    generation: u64,
    source: FileBinding,
    worker: worker::Worker,
}
struct Published {
    binding: FileBinding,
    bytes: Arc<[u8]>,
}
fn read_source(path: &Path, parent: &mut Option<Arc<Directory>>) -> Snapshot {
    if parent.is_none() {
        match Directory::for_path(path) {
            Ok((directory, _)) => *parent = Some(directory),
            Err(error) => return Snapshot::Unavailable(error.into()),
        }
    }
    if let Err(error) = parent.as_ref().unwrap().verify() {
        return Snapshot::Unavailable(error.into());
    }
    match FileBinding::capture(path) {
        Ok(binding) => match parent
            .as_ref()
            .unwrap()
            .verify()
            .and_then(|_| binding.verify())
        {
            Ok(()) => Snapshot::Source(binding),
            Err(error) => Snapshot::Unavailable(error.into()),
        },
        Err(error) => {
            if std::fs::symlink_metadata(path)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                Snapshot::Unavailable(PreviewFailure::new(
                    "input/missing",
                    "The preview input is missing; the last successful file is unchanged",
                    1,
                ))
            } else {
                Snapshot::Unavailable(error.into())
            }
        }
    }
}
fn emit_failure(state: &mut State, failure: PreviewFailure) -> PreviewResult<()> {
    state
        .failed(failure)
        .map_err(|e| PreviewFailure::io("preview/events", e))
}
fn observe(
    request: &PreviewRequest<'_>,
    parent: &mut Option<Arc<Directory>>,
    queue: &mut Latest<Snapshot>,
    active: Option<(&FileBinding, u64)>,
    state: &mut State,
) -> Option<output::FileIdentity> {
    let snapshot = read_source(request.compile.candidate, parent);
    let identity = snapshot.source().map(|source| source.identity().clone());
    let stamp = snapshot.stamp();
    if queue.stamp.as_ref() != Some(&stamp) {
        state.observe(&snapshot, queue.generation + 1);
        queue.observe(stamp, snapshot, Instant::now());
    } else if identity.is_some() {
        let stale_pending = queue
            .pending
            .as_ref()
            .and_then(|pending| pending.value.source())
            .is_some_and(|source| source.verify().is_err());
        let stale_active = active.is_some_and(|(source, generation)| {
            generation == queue.generation && source.verify().is_err()
        });
        if stale_pending || stale_active {
            queue.replace_pending(snapshot, Instant::now());
        }
    }
    identity
}

fn validate_result(
    result: &worker::WorkerResult,
    active: &Active,
    request: &PreviewRequest<'_>,
) -> PreviewResult<()> {
    if result.generation != active.generation
        || result.input_sha256 != active.source.identity().sha256
        || result.receipt.engine != *request.compile.engine
    {
        return Err(PreviewFailure::new(
            "preview/worker-binding",
            "The worker returned a different generation, input or engine identity",
            3,
        ));
    }
    if result.exit != 0 || result.receipt.status != Status::Pass {
        return Err(PreviewFailure {
            diagnostics: if result.receipt.diagnostics.is_empty() {
                PreviewFailure::new(
                    "preview/compile",
                    "Preview compilation did not pass",
                    result.exit.max(1),
                )
                .diagnostics
            } else {
                result.receipt.diagnostics.clone()
            },
            exit: result.exit.max(1),
        });
    }
    let Some(html) = &result.html else {
        return Err(PreviewFailure::new(
            "preview/worker-binding",
            "A successful worker did not return HTML",
            3,
        ));
    };
    if result.receipt.artifact.as_ref() != Some(&FileIdentity::of(html)) {
        return Err(PreviewFailure::new(
            "preview/worker-binding",
            "The worker HTML does not match its checked identity",
            3,
        ));
    }
    Ok(())
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
    let session = match output::transaction_id() {
        Ok(id) => id,
        Err(error) => {
            let mut state = State::new(String::new());
            let failure: PreviewFailure = error.into();
            let exit = failure.exit;
            let _ = state.failed(failure);
            return exit;
        }
    };
    let mut state = State::new(session.clone());
    match run_session(request, &session, &mut state) {
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
    let mut source_parent = None;
    let initial = read_source(request.compile.candidate, &mut source_parent);
    let protected: Vec<_> = initial.source().into_iter().collect();
    let mut slot = output::capture_slot(request.output, &protected)?;
    drop(protected);
    if let Some(parent) = &source_parent {
        if parent
            .real
            .join(request.compile.candidate.file_name().ok_or_else(|| {
                PreviewFailure::new("preview/input-output-alias", "Input must name a file", 2)
            })?)
            == slot.path()
        {
            return Err(PreviewFailure::new(
                "preview/input-output-alias",
                "Preview input and output must be different files",
                2,
            ));
        }
    }
    let pending = slot.pending_publications()?;
    if !pending.is_empty() {
        let mut failure = PreviewFailure::new(
            "output/recovery-required",
            "Recover the existing output publication before starting preview",
            3,
        );
        failure.diagnostics[0].evidence = json!({"pending":pending});
        return Err(failure);
    }
    let mut server = Server::bind(request.port, session)?;
    let started = state.event("started", json!({"url":server.url(),"port":server.port()}));
    if let Err(error) = started {
        server.stop();
        return Err(PreviewFailure::io("preview/events", error));
    }
    let mut queue = Latest::new();
    state.observe(&initial, 1);
    queue.observe(initial.stamp(), initial, Instant::now());
    let mut active: Option<Active> = None;
    let mut published: Option<Published> = None;
    let mut last_poll = Instant::now();
    let mut blocked = false;
    let mut exit = 0;
    if request.open {
        if let Err(failure) = open_browser(&server.url()) {
            emit_failure(state, failure)?;
        }
    }
    let work = (|| -> PreviewResult<()> {
        while !STOP.load(Ordering::Relaxed) {
            if !blocked {
                if let Err(error) = slot.verify_guard() {
                    blocked = true;
                    exit = 3;
                    published = None;
                    state.artifact = None;
                    state.artifact_available = false;
                    emit_failure(state, error.into())?;
                }
                if let Some(current) = &published {
                    if let Err(error) = current.binding.verify() {
                        blocked = true;
                        exit = 3;
                        published = None;
                        state.artifact = None;
                        state.artifact_available = false;
                        emit_failure(state, error.into())?;
                    }
                }
                if source_parent
                    .as_ref()
                    .is_some_and(|parent| parent.verify().is_err())
                {
                    blocked = true;
                    exit = 3;
                    emit_failure(
                        state,
                        PreviewFailure::new(
                            "input/parent-changed",
                            "The input parent directory changed; publication is stopped",
                            3,
                        ),
                    )?;
                }
            }
            if blocked {
                queue.pending = None;
                if let Some(mut old) = active.take() {
                    if let Err(failure) = old.worker.stop() {
                        active = Some(old);
                        return Err(failure);
                    }
                    queue.finish(old.generation);
                }
            }
            if !blocked && last_poll.elapsed() >= POLL {
                let _ = observe(
                    request,
                    &mut source_parent,
                    &mut queue,
                    active.as_ref().map(|job| (&job.source, job.generation)),
                    state,
                );
                last_poll = Instant::now();
            }
            if STOP.load(Ordering::Relaxed) {
                break;
            }
            let completed = if let Some(job) = active.as_mut() {
                match job.worker.poll() {
                    Ok(Some(result)) => Some(Ok(result)),
                    Ok(None) => None,
                    Err(failure) => Some(Err(failure)),
                }
            } else {
                None
            };
            if let Some(result) = completed {
                let mut job = active.take().unwrap();
                // A poll error need not mean the child has exited. Retain the
                // real owner until stop confirms it, including the error path.
                if result.is_err() {
                    if let Err(failure) = job.worker.stop() {
                        active = Some(job);
                        return Err(failure);
                    }
                }
                let observed = observe(
                    request,
                    &mut source_parent,
                    &mut queue,
                    Some((&job.source, job.generation)),
                    state,
                );
                // The worker has already exited. Its original descriptor must
                // still name the current source before any bytes are published.
                let current = queue.finish(job.generation);
                if current
                    && !STOP.load(Ordering::Relaxed)
                    && !blocked
                    && observed.as_ref() == Some(job.source.identity())
                    && job.source.verify().is_ok()
                {
                    match result.and_then(|result| {
                        validate_result(&result, &job, request)?;
                        Ok(result)
                    }) {
                        Err(failure) => emit_failure(state, failure)?,
                        Ok(result) => {
                            let publication =
                                (|| -> PreviewResult<Option<(Published, OutputSlot)>> {
                                    let html = result.html.as_ref().unwrap();
                                    let mut protected = vec![&job.source];
                                    protected.extend(result.resources.iter());
                                    let target = slot.path();
                                    if protected.iter().any(|source| {
                                        source.path() == target
                                            || slot.previous().is_some_and(|previous| {
                                                previous.identity().device
                                                    == source.identity().device
                                                    && previous.identity().inode
                                                        == source.identity().inode
                                            })
                                    }) {
                                        return Err(PreviewFailure::new(
                                        "output/input-alias",
                                        "Preview output cannot alias the input or a bound resource",
                                        3,
                                    ));
                                    }
                                    slot.verify_snapshot()?;
                                    if STOP.load(Ordering::Relaxed) {
                                        return Ok(None);
                                    }
                                    let receipt = output::publish_bound(&slot, html, &protected)?;
                                    if !receipt.cleanup_complete {
                                        let mut failure=PreviewFailure::new("output/recovery-required","Preview publication requires the existing output recovery command",3);
                                        failure.diagnostics[0].evidence =
                                            serde_json::to_value(&receipt).unwrap();
                                        failure.diagnostics[0].evidence["committed"] = json!(true);
                                        return Err(failure);
                                    }
                                    let binding = FileBinding::capture(&slot.path()).map_err(|error| {
                                    let mut failure:PreviewFailure=error.into();
                                    failure.diagnostics[0].evidence=json!({"committed":true,"publication":receipt,"inspection":failure.diagnostics[0].evidence});
                                    failure
                                })?;
                                    if binding.identity() != &receipt.identity
                                        || binding.bytes() != html.as_slice()
                                    {
                                        let mut failure=PreviewFailure::new("output/target-changed","Published output identity changed before preview could read it back",3);
                                        failure.diagnostics[0].evidence = json!({"committed":true,"publication":receipt,"current":binding.identity()});
                                        return Err(failure);
                                    }
                                    // Source/resource bindings were checked at publication.
                                    // A later source edit cannot undo that commit; refresh
                                    // only the exact descriptor the controller just wrote.
                                    let refreshed = slot.refresh(&binding, &[]).map_err(|error| {
                                    let mut failure:PreviewFailure=error.into();
                                    failure.diagnostics[0].evidence=json!({"committed":true,"publication":receipt,"inspection":failure.diagnostics[0].evidence});
                                    failure
                                })?;
                                    Ok(Some((
                                        Published {
                                            bytes: Arc::from(binding.bytes()),
                                            binding,
                                        },
                                        refreshed,
                                    )))
                                })();
                            let (current, refreshed) = match publication {
                                Ok(Some(publication)) => publication,
                                Ok(None) => break,
                                Err(failure) => {
                                    if !failure
                                        .diagnostics
                                        .iter()
                                        .any(|d| d.evidence["committed"] == true)
                                        && job.source.verify().is_err()
                                        && published
                                            .as_ref()
                                            .is_none_or(|current| current.binding.verify().is_ok())
                                    {
                                        let _ = observe(
                                            request,
                                            &mut source_parent,
                                            &mut queue,
                                            Some((&job.source, job.generation)),
                                            state,
                                        );
                                        if queue.generation == job.generation
                                            && queue.pending.is_none()
                                        {
                                            emit_failure(state, failure)?;
                                        }
                                    } else if failure.exit == 3 {
                                        blocked = true;
                                        exit = 3;
                                        published = None;
                                        state.artifact = None;
                                        state.artifact_available = false;
                                        emit_failure(state, failure)?;
                                    } else {
                                        return Err(failure);
                                    }
                                    continue;
                                }
                            };
                            let artifact = Artifact {
                                generation: job.generation,
                                input_sha256: job.source.identity().sha256.clone(),
                                sha256: sha256(current.binding.bytes()),
                                bytes: current.binding.bytes().len() as u64,
                            };
                            published = Some(current);
                            slot = refreshed;
                            state.status = "ready";
                            state.last_attempt_status = "pass";
                            state.last_successful_generation = job.generation;
                            state.artifact = Some(artifact);
                            state.artifact_available = true;
                            state.diagnostics.clear();
                            state
                                .event("updated", json!({}))
                                .map_err(|e| PreviewFailure::io("preview/events", e))?;
                        }
                    }
                }
            }
            if !blocked && !STOP.load(Ordering::Relaxed) {
                if let Some(pending) = queue.start(Instant::now()) {
                    match pending.value {
                        Snapshot::Unavailable(failure) => {
                            queue.finish(pending.generation);
                            emit_failure(state, failure)?;
                        }
                        Snapshot::Source(source) => {
                            if let Err(error) = source.verify() {
                                queue.finish(pending.generation);
                                let _ = observe(
                                    request,
                                    &mut source_parent,
                                    &mut queue,
                                    Some((&source, pending.generation)),
                                    state,
                                );
                                if queue.pending.is_none() {
                                    emit_failure(state, error.into())?;
                                }
                            } else {
                                if STOP.load(Ordering::Relaxed) {
                                    break;
                                }
                                match worker::launch(&request.compile, pending.generation, &source)
                                {
                                    Ok(worker) => {
                                        active = Some(Active {
                                            generation: pending.generation,
                                            source,
                                            worker,
                                        });
                                        state.status = "checking";
                                        state.last_attempt_status = "checking";
                                        state.diagnostics.clear();
                                        if STOP.load(Ordering::Relaxed) {
                                            break;
                                        }
                                        state
                                            .event("checking", json!({}))
                                            .map_err(|e| PreviewFailure::io("preview/events", e))?;
                                    }
                                    Err(failure) => {
                                        queue.finish(pending.generation);
                                        emit_failure(state, failure)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if STOP.load(Ordering::Relaxed) {
                break;
            }
            let status = serde_json::to_vec(state)
                .map_err(|error| PreviewFailure::new("preview/status", error.to_string(), 4))?;
            server.poll(
                &status,
                published.as_ref().map(|current| &current.bytes),
                blocked,
            )?;
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    })();
    if let Err(failure) = work {
        exit = failure.exit;
        let _ = state.failed(failure);
    }
    queue.pending = None;
    let listener_closed = server.stop();
    let mut worker_closed = true;
    if let Some(mut job) = active.take() {
        if let Err(failure) = job.worker.stop() {
            worker_closed = false;
            exit = failure.exit;
            let _ = state.failed(failure);
        }
    }
    let shutdown_complete = listener_closed && worker_closed;
    if !shutdown_complete {
        exit = exit.max(4);
    }
    if state.last_attempt_status == "checking" {
        state.last_attempt_status = "cancelled";
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newest_pending_replaces_intermediate_work_and_old_completion_cannot_publish() {
        let now = Instant::now();
        let mut queue = Latest::new();
        queue.observe("first".into(), "first", now);
        let first = queue.start(now + STABLE).unwrap();
        assert_eq!(first.generation, 1);
        assert!(queue.start(now + STABLE).is_none());
        queue.observe("second".into(), "second", now + STABLE);
        queue.observe("latest".into(), "latest", now + STABLE);
        assert!(queue.start(now + STABLE + STABLE).is_none());
        assert!(!queue.finish(first.generation));
        let latest = queue.start(now + STABLE + STABLE).unwrap();
        assert_eq!(latest.generation, 3);
        assert_eq!(latest.value, "latest");
        assert!(queue.finish(latest.generation));
        assert!(!queue.observe("latest".into(), "unchanged", now + STABLE));
        assert!(queue.pending.is_none());
    }
}
