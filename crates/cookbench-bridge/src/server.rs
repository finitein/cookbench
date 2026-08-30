//! Read-only bridge session state machine.

use crate::protocol::{
    Capability, ConfiguredRoot, Frame, NormalizedEvent, ProtocolError, ProtocolVersion,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerAction {
    Reply(Frame),
    Replies(Vec<Frame>),
    Configure(Vec<ConfiguredRoot>),
    Shutdown,
}

impl ServerAction {
    pub fn frames(&self) -> &[Frame] {
        match self {
            Self::Reply(frame) => std::slice::from_ref(frame),
            Self::Replies(frames) => frames,
            Self::Configure(_) => &[],
            Self::Shutdown => &[],
        }
    }
}

pub struct BridgeServer {
    capabilities: Vec<Capability>,
    negotiated: bool,
    shutdown: bool,
}

impl BridgeServer {
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self {
            capabilities,
            negotiated: false,
            shutdown: false,
        }
    }

    pub fn handle(&mut self, frame: Frame) -> Result<ServerAction, ProtocolError> {
        if self.shutdown {
            return Err(ProtocolError::Shutdown);
        }
        match frame {
            Frame::Hello { version } => self.hello(version),
            Frame::Heartbeat if self.negotiated => Ok(ServerAction::Reply(Frame::Heartbeat)),
            Frame::Configure { roots } if self.negotiated => {
                if roots.len() > 16 {
                    return Err(ProtocolError::InvalidRoot);
                }
                Ok(ServerAction::Configure(roots))
            }
            Frame::Shutdown => {
                self.shutdown = true;
                Ok(ServerAction::Shutdown)
            }
            Frame::Heartbeat | Frame::Configure { .. } => Err(ProtocolError::HandshakeRequired),
            // Event and capability frames originate at the bridge. There is no
            // request grammar for writes, prompts, approvals, or agent control.
            Frame::Capabilities { .. } | Frame::Event { .. } => {
                Err(ProtocolError::UnexpectedMessage)
            }
        }
    }

    pub fn event(&self, event: NormalizedEvent) -> Result<Frame, ProtocolError> {
        if self.shutdown {
            Err(ProtocolError::Shutdown)
        } else if !self.negotiated {
            Err(ProtocolError::HandshakeRequired)
        } else {
            Ok(Frame::Event { event })
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    fn hello(&mut self, version: u16) -> Result<ServerAction, ProtocolError> {
        if version != ProtocolVersion::CURRENT {
            return Err(ProtocolError::VersionMismatch {
                expected: ProtocolVersion::CURRENT,
                received: version,
            });
        }
        self.negotiated = true;
        Ok(ServerAction::Replies(vec![
            Frame::Hello {
                version: ProtocolVersion::CURRENT,
            },
            Frame::Capabilities {
                capabilities: self.capabilities.clone(),
            },
        ]))
    }
}
