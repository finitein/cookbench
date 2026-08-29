mod envelope;
mod spool;

use std::{
    env,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const SPOOL_ENV: &str = "COOKBENCH_HOOK_SPOOL_DIR";
const EX_USAGE: u8 = 64;
const EX_UNAVAILABLE: u8 = 69;
const EX_IOERR: u8 = 74;
const EX_TEMPFAIL: u8 = 75;

fn main() -> ExitCode {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => run_hook(None, None),
        [flag, harness]
            if flag == "--harness"
                && matches!(harness.as_str(), "codex" | "claude-code" | "claude") =>
        {
            run_hook(Some(harness), None)
        }
        [flag, harness, payload] if flag == "--harness" && harness == "codex" => {
            run_hook(Some(harness), Some(payload.as_bytes()))
        }
        [flag] if flag == "--self-test" => self_test(),
        [flag] if flag == "--version" => {
            println!("cookbench-hook {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => fail(EX_USAGE, "unsupported hook helper argument"),
    }
}

/// Accepts native Claude stdin or Codex notify argv JSON, projects it to the
/// strict metadata-only envelope, and never writes back to either agent.
fn run_hook(expected_harness: Option<&String>, argument_payload: Option<&[u8]>) -> ExitCode {
    let stdin_input;
    let input = if let Some(payload) = argument_payload {
        if payload.len() > envelope::MAX_INPUT_BYTES {
            return fail(EX_USAGE, InputError::TooLarge.diagnostic());
        }
        payload
    } else {
        stdin_input = match read_bounded_stdin() {
            Ok(input) => input,
            Err(error) => return fail(EX_USAGE, error.diagnostic()),
        };
        stdin_input.as_slice()
    };
    let parsed = if let Some(harness) = expected_harness {
        envelope::parse_native(input, harness, now_ms())
            .or_else(|_| envelope::parse(input, now_ms()))
    } else {
        envelope::parse(input, now_ms())
    };
    let envelope = match parsed {
        Ok(envelope) => envelope,
        Err(error) => return fail(EX_USAGE, error.diagnostic()),
    };
    if let Some(expected) = expected_harness {
        let expected = match expected.as_str() {
            "claude" | "claude-code" => "claude_code",
            value => value,
        };
        if envelope.event.harness != expected {
            return fail(EX_USAGE, "hook harness does not match its invocation");
        }
    }
    let spool = match env::var_os(SPOOL_ENV)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(default_spool_dir)
    {
        Some(path) => path,
        _ => return fail(EX_UNAVAILABLE, "Cookbench hook spool is unavailable"),
    };

    match spool::write_atomic(&spool, &envelope) {
        Ok(()) => ExitCode::SUCCESS,
        Err(spool::SpoolError::Missing) => {
            fail(EX_UNAVAILABLE, spool::SpoolError::Missing.diagnostic())
        }
        Err(spool::SpoolError::Full) => fail(EX_TEMPFAIL, spool::SpoolError::Full.diagnostic()),
        Err(spool::SpoolError::Io) => fail(EX_IOERR, spool::SpoolError::Io.diagnostic()),
    }
}

fn default_spool_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME").map(|home| {
            PathBuf::from(home).join("Library/Application Support/app.cookbench.desktop/hook-spool")
        })
    }
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("app.cookbench.desktop/hook-spool"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .map(|path| path.join("app.cookbench.desktop/hook-spool"))
    }
}

fn self_test() -> ExitCode {
    let started = Instant::now();
    let spool = env::temp_dir().join(format!("cookbench-hook-self-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&spool);
    if std::fs::create_dir_all(&spool).is_err() {
        return fail(EX_IOERR, "Cookbench hook self-test failed");
    }
    let envelope = match envelope::parse(
        br#"{"event_type":"tool_completed","session_id":"self-test","harness":"codex"}"#,
        now_ms(),
    ) {
        Ok(envelope) => envelope,
        Err(_) => return fail(EX_IOERR, "Cookbench hook self-test failed"),
    };
    let result = spool::write_atomic(&spool, &envelope);
    let _ = std::fs::remove_dir_all(&spool);
    if result.is_err() {
        return fail(EX_IOERR, "Cookbench hook self-test failed");
    }
    println!("self-test: passed in {} ms", started.elapsed().as_millis());
    ExitCode::SUCCESS
}

#[derive(Clone, Copy)]
enum InputError {
    TooLarge,
    ReadFailed,
}

impl InputError {
    const fn diagnostic(self) -> &'static str {
        match self {
            Self::TooLarge => "hook event exceeds the input limit",
            Self::ReadFailed => "hook event could not be read",
        }
    }
}

fn read_bounded_stdin() -> Result<Vec<u8>, InputError> {
    let mut input = Vec::with_capacity(envelope::MAX_INPUT_BYTES);
    io::stdin()
        .take((envelope::MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| InputError::ReadFailed)?;
    if input.len() > envelope::MAX_INPUT_BYTES {
        return Err(InputError::TooLarge);
    }
    Ok(input)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn fail(code: u8, diagnostic: &str) -> ExitCode {
    eprintln!("cookbench-hook: {diagnostic}");
    ExitCode::from(code)
}
