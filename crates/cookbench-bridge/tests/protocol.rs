use std::io::{BufReader, Cursor};

use cookbench_bridge::{
    protocol::{
        read_lf_frame, Capability, ConfiguredHarness, ConfiguredRoot, Frame, NormalizedEvent,
        ProtocolError, ProtocolVersion, MAX_RECORD_BYTES,
    },
    server::{BridgeServer, ServerAction},
};

#[test]
fn jsonl_frames_are_lf_terminated_and_round_trip() {
    let frame = Frame::Hello {
        version: ProtocolVersion::CURRENT,
    };
    let encoded = frame.to_jsonl().unwrap();

    assert!(encoded.ends_with(b"\n"));
    assert!(!encoded[..encoded.len() - 1].contains(&b'\n'));
    assert_eq!(Frame::from_jsonl(&encoded).unwrap(), frame);
}

#[test]
fn hello_negotiates_read_only_capabilities_and_normalized_events() {
    let mut server = BridgeServer::new(vec![
        Capability::SessionDiscovery,
        Capability::SessionParsing,
    ]);
    let action = server
        .handle(Frame::Hello {
            version: ProtocolVersion::CURRENT,
        })
        .unwrap();

    assert_eq!(
        action.frames()[0],
        Frame::Hello {
            version: ProtocolVersion::CURRENT
        }
    );
    assert_eq!(
        action.frames()[1],
        Frame::Capabilities {
            capabilities: vec![Capability::SessionDiscovery, Capability::SessionParsing],
        }
    );

    let event = NormalizedEvent::state("host:codex:opaque-session", "codex", "planning", 8)
        .with_progress(2, 5);
    let event_frame = server.event(event.clone()).unwrap();
    assert_eq!(event_frame, Frame::Event { event });
}

#[test]
fn heartbeat_and_shutdown_are_graceful() {
    let mut server = BridgeServer::new(vec![]);
    server
        .handle(Frame::Hello {
            version: ProtocolVersion::CURRENT,
        })
        .unwrap();

    assert_eq!(
        server.handle(Frame::Heartbeat).unwrap(),
        ServerAction::Reply(Frame::Heartbeat)
    );
    assert_eq!(
        server.handle(Frame::Shutdown).unwrap(),
        ServerAction::Shutdown
    );
    assert!(server.is_shutdown());
}

#[test]
fn configured_roots_are_bounded_absolute_read_only_inputs() {
    let root = ConfiguredRoot::new(ConfiguredHarness::Auto, "/custom/sessions").unwrap();
    let mut server = BridgeServer::new(vec![]);
    assert!(matches!(
        server.handle(Frame::Configure {
            roots: vec![root.clone()]
        }),
        Err(ProtocolError::HandshakeRequired)
    ));
    server
        .handle(Frame::Hello {
            version: ProtocolVersion::CURRENT,
        })
        .unwrap();
    assert_eq!(
        server
            .handle(Frame::Configure { roots: vec![root] })
            .unwrap(),
        ServerAction::Configure(vec![ConfiguredRoot::new(
            ConfiguredHarness::Auto,
            "/custom/sessions"
        )
        .unwrap()])
    );
    assert!(ConfiguredRoot::new(ConfiguredHarness::Auto, "relative").is_err());
}

#[test]
fn mismatched_versions_and_corrupt_or_oversized_input_are_rejected() {
    let mut server = BridgeServer::new(vec![]);
    assert!(matches!(
        server.handle(Frame::Hello {
            version: ProtocolVersion::CURRENT + 1
        }),
        Err(ProtocolError::VersionMismatch { .. })
    ));

    assert!(matches!(
        Frame::from_jsonl(b"not json\n"),
        Err(ProtocolError::CorruptJson(_))
    ));
    assert!(matches!(
        Frame::from_jsonl(b"{\"type\":\"shutdown\"}\r\n"),
        Err(ProtocolError::InvalidFrame(_))
    ));

    let mut input = BufReader::new(Cursor::new(vec![b'x'; MAX_RECORD_BYTES + 1]));
    assert!(matches!(
        read_lf_frame(&mut input),
        Err(ProtocolError::RecordTooLarge)
    ));
}

#[test]
fn write_or_agent_control_messages_are_not_representable() {
    let encoded = serde_json::to_string(&Frame::Capabilities {
        capabilities: vec![],
    })
    .unwrap();
    for forbidden in ["write", "prompt", "approve", "agent_control", "start_agent"] {
        assert!(!encoded.contains(forbidden));
    }
}
