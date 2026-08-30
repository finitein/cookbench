use std::io::{self, BufReader, BufWriter, Write};

use cookbench_bridge::{
    protocol::{read_lf_frame, Capability, Frame, ProtocolError},
    server::{BridgeServer, ServerAction},
    source::NativeSessionSource,
};

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--version"] {
        println!("cookbench-bridge {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if arguments != ["--stdio"] {
        eprintln!("cookbench-bridge: expected --stdio");
        std::process::exit(64);
    }
    if let Err(error) = run_stdio() {
        eprintln!("Cookbench bridge stopped: {error}");
        std::process::exit(1);
    }
}

/// Runs only on inherited standard input/output, intended for `ssh host bridge
/// --stdio`. No listener, port, prompt, approval, or agent-control command is
/// created by this binary.
fn run_stdio() -> Result<(), ProtocolError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut output = BufWriter::new(stdout.lock());
    let mut server = BridgeServer::new(vec![
        Capability::SessionDiscovery,
        Capability::SessionParsing,
    ]);
    let mut source: Option<NativeSessionSource> = None;

    loop {
        let frame = Frame::from_jsonl(&read_lf_frame(&mut input)?)?;
        match server.handle(frame)? {
            ServerAction::Reply(Frame::Heartbeat) => {
                if let Some(source) = source.as_mut() {
                    emit_events(&mut output, &server, source)?;
                }
                write_frame(&mut output, &Frame::Heartbeat)?;
            }
            ServerAction::Reply(frame) => {
                write_frame(&mut output, &frame)?;
            }
            ServerAction::Replies(frames) => {
                for frame in frames {
                    write_frame(&mut output, &frame)?;
                }
                write_frame(&mut output, &Frame::Heartbeat)?;
            }
            ServerAction::Configure(roots) => {
                source = Some(if roots.is_empty() {
                    NativeSessionSource::default()
                } else {
                    NativeSessionSource::from_configured_roots(roots)
                });
                emit_events(
                    &mut output,
                    &server,
                    source.as_mut().expect("configured source was just created"),
                )?;
                write_frame(&mut output, &Frame::Heartbeat)?;
            }
            ServerAction::Shutdown => {
                write_frame(&mut output, &Frame::Shutdown)?;
                return Ok(());
            }
        }
    }
}

fn emit_events(
    output: &mut impl Write,
    server: &BridgeServer,
    source: &mut NativeSessionSource,
) -> Result<(), ProtocolError> {
    for event in source.poll() {
        write_frame(output, &server.event(event)?)?;
    }
    Ok(())
}

fn write_frame(output: &mut impl Write, frame: &Frame) -> Result<(), ProtocolError> {
    output
        .write_all(&frame.to_jsonl()?)
        .map_err(ProtocolError::Io)?;
    output.flush().map_err(ProtocolError::Io)
}
