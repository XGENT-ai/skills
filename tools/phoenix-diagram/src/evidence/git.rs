// Read-only local Git access. Every child gets an argument vector (no shell),
// a scrubbed GIT_* environment that refuses every transport, and its own
// process group so a timeout also ends anything it started.
use std::ffi::OsString;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The Git program and per-command time limit.
#[derive(Clone, Debug)]
pub struct GitRunner {
    pub program: OsString,
    pub timeout: Duration,
}

impl Default for GitRunner {
    fn default() -> Self {
        Self {
            program: "git".into(),
            timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Failure {
    Unavailable,
    Timeout,
}

pub(crate) struct Output {
    pub success: bool,
    pub stdout: Vec<u8>,
}

fn drain(mut source: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = source.read_to_end(&mut bytes);
        bytes
    })
}

fn kill_group(child: &mut Child) {
    // The child leads its own group; a negative pid signals the whole group.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
}

impl GitRunner {
    pub(crate) fn run(
        &self,
        root: &Path,
        args: &[&str],
        input: Option<Vec<u8>>,
    ) -> Result<Output, Failure> {
        let mut command = Command::new(&self.program);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                command.env_remove(key);
            }
        }
        // Git 2.45+ honors NO_LAZY_FETCH; an empty protocol allow-list makes
        // any lazy fetch fail in the transport layer on every version.
        command
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_ALLOW_PROTOCOL", "")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .arg("--no-replace-objects")
            .arg("-C")
            .arg(root)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = command.spawn().map_err(|_| Failure::Unavailable)?;
        let stdout = drain(child.stdout.take().expect("piped stdout"));
        let stderr = drain(child.stderr.take().expect("piped stderr"));
        let writer = input.map(|bytes| {
            let mut stdin = child.stdin.take().expect("piped stdin");
            thread::spawn(move || {
                let _ = stdin.write_all(&bytes);
            })
        });
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                _ => {
                    // A process that left the group could hold the pipes open,
                    // so the readers are not joined here.
                    kill_group(&mut child);
                    return Err(Failure::Timeout);
                }
            }
        };
        if let Some(writer) = writer {
            let _ = writer.join();
        }
        let _ = stderr.join();
        Ok(Output {
            success: status.success(),
            stdout: stdout.join().unwrap_or_default(),
        })
    }
}

/// `git version` → (major, minor), or None when the banner is not understood.
pub(crate) fn version(banner: &[u8]) -> Option<(u32, u32)> {
    let text = std::str::from_utf8(banner).ok()?;
    let mut numbers = text
        .trim()
        .strip_prefix("git version ")?
        .split(|c: char| !c.is_ascii_digit())
        .map(str::parse::<u32>);
    Some((numbers.next()?.ok()?, numbers.next()?.ok()?))
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_banners_from_upstream_and_apple_git_parse() {
        assert_eq!(version(b"git version 2.53.0\n"), Some((2, 53)));
        assert_eq!(
            version(b"git version 2.39.5 (Apple Git-154)\n"),
            Some((2, 39))
        );
        assert_eq!(version(b"git version 2.45.0.windows.1\n"), Some((2, 45)));
        assert_eq!(version(b"hub version 2.14\n"), None);
    }
}
