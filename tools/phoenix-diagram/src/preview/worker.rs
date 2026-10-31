//! Same-executable snapshot compilation. Only the controller publishes files.
use super::{PreviewFailure, PreviewResult};
use crate::{
    check::FileIdentity,
    delivery::{prepare, CompileRequest, EngineIdentity, Status, ValidationReceipt},
    input::{sha256, CompileOptions, DiagramType, InputContract, Quality},
    output::FileBinding,
    render::EmbeddedAssets,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    generation: u64,
    kind: DiagramType,
    input_base64: String,
    input_contract: Option<InputContract>,
    quality: Option<Quality>,
    viewer_revision: String,
    repo_root: Option<PathBuf>,
    brand_cache: Option<PathBuf>,
    engine: EngineIdentity,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    schema_version: u32,
    generation: u64,
    input_sha256: String,
    receipt: ValidationReceipt,
    html_base64: Option<String>,
}
pub struct WorkerResult {
    pub generation: u64,
    pub input_sha256: String,
    pub receipt: ValidationReceipt,
    pub html: Option<Vec<u8>>,
    pub resources: Vec<FileBinding>,
    pub exit: u8,
}
pub struct Worker {
    child: Child,
    exchange: Option<JoinHandle<io::Result<Vec<u8>>>>,
    generation: u64,
    input_sha256: String,
    engine: EngineIdentity,
    resources: Vec<FileBinding>,
    executable: FileBinding,
    finished: bool,
}
fn io_failure(error: impl std::fmt::Display) -> PreviewFailure {
    PreviewFailure::new("preview/worker", error.to_string(), 4)
}
pub fn launch(
    request: &CompileRequest<'_>,
    generation: u64,
    source: &FileBinding,
) -> PreviewResult<Worker> {
    source.verify().map_err(PreviewFailure::from)?;
    let executable = FileBinding::capture(&std::env::current_exe().map_err(io_failure)?)
        .map_err(PreviewFailure::from)?;
    if FileIdentity::of(executable.bytes()) != request.engine.binary {
        return Err(PreviewFailure::new(
            "preview/worker-engine",
            "Preview requires the current executable's engine identity",
            3,
        ));
    }
    // Keep the resources' original descriptors in the controller across worker
    // execution. Compilation remains in the child; this only captures bindings.
    let mut resources = vec![];
    if let Ok(input) = crate::input::parse_input(source.bytes(), request.kind, request.options) {
        let cache = request
            .brand_cache
            .map(Path::to_owned)
            .unwrap_or_else(crate::brands::default_cache);
        let brands =
            crate::brands::prepare(&input, &cache).map_err(|diagnostics| PreviewFailure {
                exit: if diagnostics.iter().any(|d| d.code == "brand/cache-missing") {
                    5
                } else {
                    1
                },
                diagnostics,
            })?;
        resources =
            crate::brands::resource_bindings(&brands, &cache).map_err(|error| PreviewFailure {
                diagnostics: vec![error.diagnostic("")],
                exit: error.exit_code(),
            })?;
    }
    let envelope = Envelope {
        schema_version: 1,
        generation,
        kind: request.kind,
        input_base64: STANDARD.encode(source.bytes()),
        input_contract: request.options.input_contract,
        quality: request.options.quality,
        viewer_revision: request.options.viewer_revision.clone(),
        repo_root: request.repo_root.map(Path::to_owned),
        brand_cache: request.brand_cache.map(Path::to_owned),
        engine: request.engine.clone(),
    };
    let bytes = serde_json::to_vec(&envelope).map_err(io_failure)?;
    let mut child = Command::new(executable.path())
        .arg("__preview-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(io_failure)?;
    let mut stdin = child.stdin.take().expect("piped worker stdin");
    let mut stdout = child.stdout.take().expect("piped worker stdout");
    let exchange = thread::spawn(move || {
        stdin.write_all(&bytes)?;
        drop(stdin);
        let mut response = vec![];
        stdout.read_to_end(&mut response)?;
        Ok(response)
    });
    let worker = Worker {
        child,
        exchange: Some(exchange),
        generation,
        input_sha256: sha256(source.bytes()),
        engine: request.engine.clone(),
        resources,
        executable,
        finished: false,
    };
    source.verify().map_err(PreviewFailure::from)?;
    Ok(worker)
}
impl Worker {
    pub fn poll(&mut self) -> PreviewResult<Option<WorkerResult>> {
        let Some(status) = self.child.try_wait().map_err(io_failure)? else {
            return Ok(None);
        };
        self.finished = true;
        let exit = status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .ok_or_else(|| {
                PreviewFailure::new(
                    "preview/worker-exit",
                    "Preview worker terminated without a normal result",
                    4,
                )
            })?;
        let bytes = self
            .exchange
            .take()
            .ok_or_else(|| io_failure("Preview worker result was already consumed"))?
            .join()
            .map_err(|_| io_failure("Preview worker pipe reader failed"))?
            .map_err(io_failure)?;
        let response: Response = serde_json::from_slice(&bytes).map_err(io_failure)?;
        self.executable.verify().map_err(PreviewFailure::from)?;
        for resource in &self.resources {
            resource.verify().map_err(PreviewFailure::from)?;
        }
        let html = response
            .html_base64
            .as_deref()
            .map(|encoded| {
                let bytes = STANDARD.decode(encoded).map_err(io_failure)?;
                if STANDARD.encode(&bytes) != encoded {
                    return Err(io_failure("Non-canonical preview worker HTML"));
                }
                Ok(bytes)
            })
            .transpose()?;
        if response.schema_version != 1
            || response.generation != self.generation
            || response.input_sha256 != self.input_sha256
            || response.receipt.engine != self.engine
            || response.receipt.schema_version != 1
            || response.receipt.tool != "phoenix-diagram"
            || response.receipt.tool_version != crate::TOOL_VERSION
            || response.receipt.command != "validate"
            || response.receipt.candidate.is_some()
            || match (&html, response.receipt.status) {
                (Some(bytes), Status::Pass) => {
                    exit != 0
                        || response.receipt.artifact.as_ref() != Some(&FileIdentity::of(bytes))
                }
                (None, Status::Failed | Status::Unavailable) => exit == 0,
                _ => true,
            }
        {
            return Err(PreviewFailure::new(
                "preview/worker-binding",
                "Worker generation, input, engine or artifact binding differs",
                3,
            ));
        }
        Ok(Some(WorkerResult {
            generation: response.generation,
            input_sha256: response.input_sha256,
            receipt: response.receipt,
            html,
            resources: std::mem::take(&mut self.resources),
            exit,
        }))
    }
    pub fn stop(&mut self) -> PreviewResult<()> {
        if self.finished {
            return self.finish_exchange();
        }
        if self.child.try_wait().map_err(io_failure)?.is_some() {
            self.finished = true;
            return self.finish_exchange();
        }
        self.signal(libc::SIGTERM)?;
        if !self.wait(Duration::from_millis(3000))? {
            self.signal(libc::SIGKILL)?;
            if !self.wait(Duration::from_millis(750))? {
                return Err(PreviewFailure::new(
                    "preview/worker-stop",
                    "Owned preview worker did not exit after forced stop",
                    4,
                ));
            }
        }
        self.finish_exchange()
    }
    fn signal(&self, signal: libc::c_int) -> PreviewResult<()> {
        if unsafe { libc::kill(-(self.child.id() as libc::pid_t), signal) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(io_failure(error));
            }
        }
        Ok(())
    }
    fn wait(&mut self, duration: Duration) -> PreviewResult<bool> {
        let deadline = Instant::now() + duration;
        loop {
            if self.child.try_wait().map_err(io_failure)?.is_some() {
                self.finished = true;
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn finish_exchange(&mut self) -> PreviewResult<()> {
        if let Some(exchange) = self.exchange.take() {
            // BrokenPipe is expected when cancellation interrupts stdin delivery.
            let _ = exchange
                .join()
                .map_err(|_| io_failure("Preview worker pipe reader failed"))?;
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Private stdin/stdout protocol; it neither reopens the source nor publishes.
pub fn serve(engine: &EngineIdentity) -> u8 {
    let result = (|| -> PreviewResult<u8> {
        let envelope: Envelope = serde_json::from_reader(io::stdin().lock()).map_err(io_failure)?;
        let bytes = STANDARD
            .decode(&envelope.input_base64)
            .map_err(io_failure)?;
        if envelope.schema_version != 1
            || envelope.generation == 0
            || envelope.engine != *engine
            || STANDARD.encode(&bytes) != envelope.input_base64
        {
            return Err(PreviewFailure::new(
                "preview/worker-envelope",
                "Unsupported private worker envelope or engine",
                2,
            ));
        }
        let options = CompileOptions {
            input_contract: envelope.input_contract,
            quality: envelope.quality,
            viewer_revision: envelope.viewer_revision,
        };
        crate::output::preview_worker_checkpoint(envelope.generation, &sha256(&bytes))
            .map_err(io_failure)?;
        let assets = EmbeddedAssets::builtin();
        let prepared = prepare::prepare_bytes(
            &CompileRequest {
                kind: envelope.kind,
                candidate: Path::new("<preview-snapshot>"),
                options: &options,
                engine,
                assets: &assets,
                repo_root: envelope.repo_root.as_deref(),
                brand_cache: envelope.brand_cache.as_deref(),
            },
            &bytes,
        );
        let exit = prepared.receipt.exit_code();
        if let Some(compiled) = prepared.compiled.as_ref() {
            for resource in &compiled.resources {
                resource.verify().map_err(PreviewFailure::from)?;
            }
        }
        let response = Response {
            schema_version: 1,
            generation: envelope.generation,
            input_sha256: sha256(&bytes),
            receipt: prepared.receipt,
            html_base64: prepared
                .compiled
                .as_ref()
                .map(|compiled| STANDARD.encode(&compiled.html)),
        };
        serde_json::to_writer(io::stdout().lock(), &response).map_err(io_failure)?;
        Ok(exit)
    })();
    match result {
        Ok(exit) => exit,
        Err(failure) => {
            let _ = serde_json::to_writer(
                io::stdout().lock(),
                &serde_json::json!({"status":"failed","diagnostics":failure.diagnostics}),
            );
            failure.exit
        }
    }
}
