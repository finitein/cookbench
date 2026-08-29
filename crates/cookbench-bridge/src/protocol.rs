//! Versioned, bounded JSON Lines protocol used only over SSH standard I/O.

use std::{fmt, io::BufRead};

use serde::{Deserialize, Serialize};

pub const MAX_RECORD_BYTES: usize = 64 * 1024;

pub struct ProtocolVersion;

impl ProtocolVersion {
    pub const CURRENT: u16 = 1;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    SessionDiscovery,
    SessionParsing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedEvent {
    /// Opaque deterministic stove identity, never a prompt or transcript.
    pub stove_key: String,
    pub harness: String,
    pub state: NormalizedState,
    pub sequence: u64,
}

impl NormalizedEvent {
    pub fn state(
        stove_key: impl Into<String>,
        harness: impl Into<String>,
        state: &str,
        sequence: u64,
    ) -> Self {
        Self {
            stove_key: stove_key.into(),
            harness: harness.into(),
            state: NormalizedState::from_wire(state),
            sequence,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedState {
    Starting,
    Planning,
    Cooking,
    NeedsHuman,
    Cooked,
    Failed,
    Disconnected,
}

impl NormalizedState {
    fn from_wire(value: &str) -> Self {
        match value {
            "starting" => Self::Starting,
            "planning" => Self::Planning,
            "cooking" => Self::Cooking,
            "needs_human" => Self::NeedsHuman,
            "cooked" => Self::Cooked,
            "failed" => Self::Failed,
            _ => Self::Disconnected,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    Hello { version: u16 },
    Capabilities { capabilities: Vec<Capability> },
    Event { event: NormalizedEvent },
    Heartbeat,
    Shutdown,
}

impl Frame {
    pub fn to_jsonl(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut bytes = serde_json::to_vec(self).map_err(ProtocolError::Encode)?;
        if bytes.len() + 1 > MAX_RECORD_BYTES {
            return Err(ProtocolError::RecordTooLarge);
        }
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn from_jsonl(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(ProtocolError::RecordTooLarge);
        }
        if bytes.last() != Some(&b'\n')
            || bytes[..bytes.len().saturating_sub(1)].contains(&b'\n')
            || bytes.contains(&b'\r')
        {
            return Err(ProtocolError::InvalidFrame(
                "records must contain exactly one LF terminator",
            ));
        }
        let body = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .map_err(|_| ProtocolError::InvalidFrame("record is not UTF-8"))?;
        serde_json::from_str(body).map_err(ProtocolError::CorruptJson)
    }
}

/// Reads one line without allocating more than the protocol record limit.
pub fn read_lf_frame(reader: &mut impl BufRead) -> Result<Vec<u8>, ProtocolError> {
    let mut frame = Vec::with_capacity(256);
    let mut byte = [0_u8; 1];
    while frame.len() <= MAX_RECORD_BYTES {
        match reader.read(&mut byte).map_err(ProtocolError::Io)? {
            0 if frame.is_empty() => return Err(ProtocolError::EndOfStream),
            0 => return Err(ProtocolError::InvalidFrame("truncated record")),
            _ => {
                frame.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(frame);
                }
            }
        }
    }
    Err(ProtocolError::RecordTooLarge)
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(std::io::Error),
    Encode(serde_json::Error),
    CorruptJson(serde_json::Error),
    InvalidFrame(&'static str),
    RecordTooLarge,
    EndOfStream,
    VersionMismatch { expected: u16, received: u16 },
    UnexpectedMessage,
    HandshakeRequired,
    Shutdown,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "standard I/O failure: {error}"),
            Self::Encode(error) => write!(f, "could not encode bridge frame: {error}"),
            Self::CorruptJson(error) => write!(f, "corrupt bridge JSON: {error}"),
            Self::InvalidFrame(reason) => write!(f, "invalid bridge frame: {reason}"),
            Self::RecordTooLarge => write!(f, "bridge record exceeds {MAX_RECORD_BYTES} bytes"),
            Self::EndOfStream => write!(f, "bridge standard input closed"),
            Self::VersionMismatch { expected, received } => write!(
                f,
                "bridge protocol version {received} is incompatible with {expected}"
            ),
            Self::UnexpectedMessage => write!(f, "bridge received a disallowed message"),
            Self::HandshakeRequired => write!(f, "bridge hello handshake is required"),
            Self::Shutdown => write!(f, "bridge is shut down"),
        }
    }
}

impl std::error::Error for ProtocolError {}
