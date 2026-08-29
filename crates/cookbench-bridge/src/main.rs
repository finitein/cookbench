use std::io::{self, BufReader, BufWriter, Write};

use cookbench_bridge::{
    protocol::{read_lf_frame, Capability, Frame, ProtocolError},
    server::{BridgeServer, ServerAction},
};

fn main() {
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

    loop {
        let frame = Frame::from_jsonl(&read_lf_frame(&mut input)?)?;
        match server.handle(frame)? {
            ServerAction::Reply(frame) => write_frame(&mut output, &frame)?,
            ServerAction::Replies(frames) => {
                for frame in frames {
                    write_frame(&mut output, &frame)?;
                }
            }
            ServerAction::Shutdown => {
                write_frame(&mut output, &Frame::Shutdown)?;
                return Ok(());
            }
        }
    }
}

fn write_frame(output: &mut impl Write, frame: &Frame) -> Result<(), ProtocolError> {
    output
        .write_all(&frame.to_jsonl()?)
        .map_err(ProtocolError::Io)?;
    output.flush().map_err(ProtocolError::Io)
}
