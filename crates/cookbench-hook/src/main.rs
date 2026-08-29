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
        [] => run_hook(),
        [flag] if flag == "--self-test" => self_test(),
        [flag] if flag == "--version" => {
            println!("cookbench-hook {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => fail(EX_USAGE, "unsupported hook helper argument"),
    }
}

fn run_hook() -> ExitCode {
    let input = match read_bounded_stdin() {
        Ok(input) => input,
        Err(error) => return fail(EX_USAGE, error.diagnostic()),
    };
    let envelope = match envelope::parse(&input, now_ms()) {
        Ok(envelope) => envelope,
        Err(error) => return fail(EX_USAGE, error.diagnostic()),
    };
    let spool = match env::var_os(SPOOL_ENV) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
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
