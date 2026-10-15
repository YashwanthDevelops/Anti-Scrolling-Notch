//! The verified Codex CLI 0.157.1 observing-hook boundary.
//!
//! The disposable loopback probe observed four event names, and one accepted
//! identity capture recorded session/turn field presence for those events.
//! Legacy wire v1 stays event-only; v2 forwards only bounded session/turn IDs.
//! Prompt text, paths, transcript data, and tool payloads never cross the pipe.
//! This does not claim a complete Codex hook schema or support for other
//! Codex surfaces.

use serde::{Deserialize, Serialize};

pub const WIRE_VERSION: u8 = 1;
pub const LIFECYCLE_WIRE_VERSION: u8 = 2;
pub const MAX_HOOK_INPUT_BYTES: usize = 1 << 20;
pub const MAX_WIRE_BYTES: usize = 1_200;
pub const MAX_ID_BYTES: usize = 512;

include!(concat!(env!("OUT_DIR"), "/codex_hook_event.rs"));

/// App-internal wire message. It carries only a verified event discriminator;
/// it is not a Codex hook input/output schema and cannot carry hook decisions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexHookObservation {
    pub wire_version: u8,
    pub event: CodexHookEvent,
}

impl CodexHookObservation {
    pub const fn new(event: CodexHookEvent) -> Self {
        Self {
            wire_version: WIRE_VERSION,
            event,
        }
    }
}

/// Session and turn identity fields captured from the pinned CLI hook schema.
/// This exact v2 message deliberately has no prompt, cwd, transcript, or raw
/// payload fields. Missing identities remain missing; callers must not infer
/// them from event order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CodexHookLifecycleObservation {
    pub wire_version: u8,
    pub event: CodexHookEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    TooLarge,
    InvalidJson,
    NotObject,
    MissingEventName,
    UnsupportedEvent,
    UnsupportedWireVersion,
    InvalidWireMessage,
    InvalidIdentifier,
    UnexpectedIdentifier,
}

/// Convert stdin into the legacy anonymous event-only observation.
pub fn parse_hook_payload(raw: &[u8]) -> Result<CodexHookObservation, ParseError> {
    if raw.len() > MAX_HOOK_INPUT_BYTES {
        return Err(ParseError::TooLarge);
    }
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| ParseError::InvalidJson)?;
    let object = value.as_object().ok_or(ParseError::NotObject)?;
    let event_name = object
        .get("hook_event_name")
        .and_then(serde_json::Value::as_str)
        .ok_or(ParseError::MissingEventName)?;
    let event = CodexHookEvent::parse(event_name).ok_or(ParseError::UnsupportedEvent)?;
    Ok(CodexHookObservation::new(event))
}

/// Validate a message received over the app-private named pipe.
pub fn parse_wire_message(raw: &[u8]) -> Result<CodexHookObservation, ParseError> {
    if raw.len() > MAX_WIRE_BYTES {
        return Err(ParseError::TooLarge);
    }
    let message: CodexHookObservation =
        serde_json::from_slice(raw).map_err(|_| ParseError::InvalidWireMessage)?;
    if message.wire_version != WIRE_VERSION {
        return Err(ParseError::UnsupportedWireVersion);
    }
    Ok(message)
}

/// Project the captured lifecycle identity fields from hook stdin. The event
/// and field presence are the only values interpreted; all other input is
/// discarded before crossing the pipe.
pub fn parse_hook_lifecycle_payload(
    raw: &[u8],
) -> Result<CodexHookLifecycleObservation, ParseError> {
    if raw.len() > MAX_HOOK_INPUT_BYTES {
        return Err(ParseError::TooLarge);
    }
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| ParseError::InvalidJson)?;
    let object = value.as_object().ok_or(ParseError::NotObject)?;
    let event_name = object
        .get("hook_event_name")
        .and_then(serde_json::Value::as_str)
        .ok_or(ParseError::MissingEventName)?;
    let event = CodexHookEvent::parse(event_name).ok_or(ParseError::UnsupportedEvent)?;
    let session_id = read_id(object.get("session_id"))?;
    let turn_id = read_id(object.get("turn_id"))?;
    validate_lifecycle_ids(event, session_id.as_deref(), turn_id.as_deref())?;
    Ok(CodexHookLifecycleObservation {
        wire_version: LIFECYCLE_WIRE_VERSION,
        event,
        session_id,
        turn_id,
    })
}

/// Decode either the legacy anonymous v1 message or the exact identity v2
/// message. Legacy messages stay anonymous and remain usable during upgrades.
pub fn parse_wire_lifecycle_message(
    raw: &[u8],
) -> Result<CodexHookLifecycleObservation, ParseError> {
    if raw.len() > MAX_WIRE_BYTES {
        return Err(ParseError::TooLarge);
    }
    let version = serde_json::from_slice::<serde_json::Value>(raw)
        .map_err(|_| ParseError::InvalidWireMessage)?
        .get("wire_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or(ParseError::InvalidWireMessage)?;
    match version {
        value if value == u64::from(WIRE_VERSION) => {
            let legacy = parse_wire_message(raw)?;
            Ok(CodexHookLifecycleObservation {
                wire_version: WIRE_VERSION,
                event: legacy.event,
                session_id: None,
                turn_id: None,
            })
        }
        value if value == u64::from(LIFECYCLE_WIRE_VERSION) => {
            let message: CodexHookLifecycleObservation =
                serde_json::from_slice(raw).map_err(|_| ParseError::InvalidWireMessage)?;
            if message.wire_version != LIFECYCLE_WIRE_VERSION {
                return Err(ParseError::UnsupportedWireVersion);
            }
            validate_lifecycle_ids(
                message.event,
                message.session_id.as_deref(),
                message.turn_id.as_deref(),
            )?;
            Ok(message)
        }
        _ => Err(ParseError::UnsupportedWireVersion),
    }
}

fn read_id(value: Option<&serde_json::Value>) -> Result<Option<String>, ParseError> {
    let Some(value) = value else { return Ok(None) };
    let id = value.as_str().ok_or(ParseError::InvalidIdentifier)?;
    if id.is_empty() || id.len() > MAX_ID_BYTES || id.chars().any(char::is_control) {
        return Err(ParseError::InvalidIdentifier);
    }
    Ok(Some(id.to_owned()))
}

fn validate_lifecycle_ids(
    event: CodexHookEvent,
    session_id: Option<&str>,
    turn_id: Option<&str>,
) -> Result<(), ParseError> {
    if turn_id.is_some()
        && (session_id.is_none()
            || !matches!(
                event,
                CodexHookEvent::UserPromptSubmit | CodexHookEvent::Stop
            ))
    {
        return Err(ParseError::UnexpectedIdentifier);
    }
    for id in [session_id, turn_id].into_iter().flatten() {
        if id.is_empty() || id.len() > MAX_ID_BYTES || id.chars().any(char::is_control) {
            return Err(ParseError::InvalidIdentifier);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    #[test]
    fn generated_event_type_matches_the_accepted_runtime_capture() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = fs::read_to_string(
            repo_root.join("tests/compat/codex-hooks/captured-projections.jsonl"),
        )
        .expect("read accepted sanitized hook projection");
        let captured = fixture
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["hook_event_name"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        let generated = CodexHookEvent::VERIFIED
            .iter()
            .map(|event| event.as_str().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(generated, captured);
        assert_eq!(CODEX_HOOK_SCHEMA_CLI_VERSION, "0.157.1");
        assert_eq!(
            CODEX_HOOK_SCHEMA_SOURCE_COMMIT,
            "36650394c5b38c2990ccf2a3457165ca3e9d9726"
        );
    }

    #[test]
    fn accepts_only_the_four_loopback_observed_event_names() {
        let cases = [
            (
                r#"{"hook_event_name":"SessionStart","source":"startup"}"#,
                CodexHookEvent::SessionStart,
            ),
            (
                r#"{"hook_event_name":"UserPromptSubmit","prompt":"redacted"}"#,
                CodexHookEvent::UserPromptSubmit,
            ),
            (r#"{"hook_event_name":"Stop"}"#, CodexHookEvent::Stop),
            (
                r#"{"hook_event_name":"SessionEnd","reason":"other"}"#,
                CodexHookEvent::SessionEnd,
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(
                parse_hook_payload(raw.as_bytes()).unwrap(),
                CodexHookObservation::new(expected)
            );
        }
    }

    #[test]
    fn prompt_and_arbitrary_input_fields_never_cross_the_adapter_boundary() {
        let raw = br#"{"hook_event_name":"UserPromptSubmit","prompt":"private prompt","cwd":"C:\\private","session_id":"private-id"}"#;
        let message = parse_hook_payload(raw).unwrap();
        let wire = serde_json::to_string(&message).unwrap();
        assert_eq!(message.event, CodexHookEvent::UserPromptSubmit);
        assert!(!wire.contains("private"));
        assert!(!wire.contains("session_id"));
        assert_eq!(wire, r#"{"wire_version":1,"event":"UserPromptSubmit"}"#);
    }

    #[test]
    fn unobserved_events_remain_unsupported() {
        for name in [
            "PreToolUse",
            "PostToolUse",
            "PreCompact",
            "PostCompact",
            "SubagentStart",
            "SubagentStop",
            "Interrupt",
            "PermissionRequest",
        ] {
            let raw = format!(r#"{{"hook_event_name":"{name}"}}"#);
            assert_eq!(
                parse_hook_payload(raw.as_bytes()),
                Err(ParseError::UnsupportedEvent),
                "{name}"
            );
        }
    }

    #[test]
    fn malformed_or_incomplete_input_is_a_neutral_parse_failure() {
        for raw in [
            &b""[..],
            &b"not-json"[..],
            &b"[]"[..],
            &br#"{"source":"startup"}"#[..],
            &br#"{"hook_event_name":7}"#[..],
        ] {
            assert!(parse_hook_payload(raw).is_err());
        }
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend_from_slice(br#"{"hook_event_name":"SessionStart"}"#);
        assert_eq!(
            parse_hook_payload(&bom).unwrap().event,
            CodexHookEvent::SessionStart
        );
        assert_eq!(
            parse_hook_payload(&vec![b' '; MAX_HOOK_INPUT_BYTES + 1]),
            Err(ParseError::TooLarge)
        );
    }

    #[test]
    fn wire_decoder_requires_known_version_and_exact_internal_shape() {
        let valid = br#"{"wire_version":1,"event":"Stop"}"#;
        assert_eq!(
            parse_wire_message(valid).unwrap(),
            CodexHookObservation::new(CodexHookEvent::Stop)
        );
        assert_eq!(
            parse_wire_message(br#"{"wire_version":2,"event":"Stop"}"#),
            Err(ParseError::UnsupportedWireVersion)
        );
        assert!(parse_wire_message(br#"{"wire_version":1,"event":"Stop","prompt":"x"}"#).is_err());
        assert_eq!(
            parse_wire_message(&vec![b' '; MAX_WIRE_BYTES + 1]),
            Err(ParseError::TooLarge)
        );
    }

    #[test]
    fn captured_identities_are_forwarded_in_a_bounded_exact_v2_message() {
        let raw = br#"{"hook_event_name":"UserPromptSubmit","session_id":"session:82dd70e639fd8dce","turn_id":"turn:4dab42ff6cd56c9a","prompt":"secret","cwd":"C:\\private","transcript_path":"private"}"#;
        let message = parse_hook_lifecycle_payload(raw).unwrap();
        assert_eq!(message.wire_version, LIFECYCLE_WIRE_VERSION);
        assert_eq!(message.event, CodexHookEvent::UserPromptSubmit);
        assert_eq!(
            message.session_id.as_deref(),
            Some("session:82dd70e639fd8dce")
        );
        assert_eq!(message.turn_id.as_deref(), Some("turn:4dab42ff6cd56c9a"));
        let wire = serde_json::to_string(&message).unwrap();
        assert!(wire.len() <= MAX_WIRE_BYTES);
        assert!(wire.contains("session:82dd70e639fd8dce"));
        assert!(wire.contains("turn:4dab42ff6cd56c9a"));
        assert!(!wire.contains("secret"));
        assert!(!wire.contains("cwd"));
        assert!(!wire.contains("transcript"));
        assert_eq!(
            parse_wire_lifecycle_message(wire.as_bytes()).unwrap(),
            message
        );
        let recorder_wrapper =
            br#"{"hook_event_name":"SessionStart","identities":{"session_id":"recorder-only"}}"#;
        let anonymous = parse_hook_lifecycle_payload(recorder_wrapper).unwrap();
        assert_eq!(anonymous.session_id, None);
    }

    #[test]
    fn lifecycle_wire_accepts_legacy_v1_as_anonymous_and_rejects_unknown_fields() {
        let legacy = br#"{"wire_version":1,"event":"Stop"}"#;
        let decoded = parse_wire_lifecycle_message(legacy).unwrap();
        assert_eq!(decoded.wire_version, WIRE_VERSION);
        assert_eq!(decoded.event, CodexHookEvent::Stop);
        assert_eq!(decoded.session_id, None);
        assert_eq!(decoded.turn_id, None);
        assert!(parse_wire_lifecycle_message(
            br#"{"wire_version":2,"event":"Stop","prompt":"private"}"#
        )
        .is_err());
    }

    #[test]
    fn lifecycle_projection_rejects_malformed_or_unsupported_identity_fields() {
        for raw in [
            br#"{"hook_event_name":"SessionStart","session_id":""}"#.as_slice(),
            br#"{"hook_event_name":"SessionStart","session_id":"bad\nvalue"}"#.as_slice(),
            br#"{"hook_event_name":"SessionStart","session_id":7}"#.as_slice(),
            br#"{"hook_event_name":"SessionStart","turn_id":"turn:x"}"#.as_slice(),
        ] {
            assert!(
                parse_hook_lifecycle_payload(raw).is_err(),
                "accepted malformed identity input: {}",
                String::from_utf8_lossy(raw)
            );
        }

        let too_long = "x".repeat(MAX_ID_BYTES + 1);
        let raw = format!(r#"{{"hook_event_name":"SessionStart","session_id":"{too_long}"}}"#);
        assert_eq!(
            parse_hook_lifecycle_payload(raw.as_bytes()),
            Err(ParseError::InvalidIdentifier)
        );
    }

    #[test]
    fn captured_session_and_turn_field_presence_matches_pinned_projection() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = fs::read_to_string(
            repo_root.join("tests/compat/codex-hooks/captured-identities-cli-0.157.1.jsonl"),
        )
        .expect("read accepted identity capture");
        for line in fixture.lines().filter(|line| !line.trim().is_empty()) {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            // The capture is a sanitized recorder envelope, not raw Codex
            // input. Project only its observed identity values into the
            // root-level fields declared by the pinned CLI schema.
            let identities = value.get("identities").unwrap();
            let mut synthetic = serde_json::Map::new();
            synthetic.insert(
                "hook_event_name".into(),
                value.get("hook_event_name").unwrap().clone(),
            );
            if let Some(id) = identities.get("session_id") {
                synthetic.insert("session_id".into(), id.clone());
            }
            if let Some(id) = identities.get("turn_id") {
                synthetic.insert("turn_id".into(), id.clone());
            }
            let synthetic = serde_json::Value::Object(synthetic);
            let observation =
                parse_hook_lifecycle_payload(serde_json::to_string(&synthetic).unwrap().as_bytes())
                    .unwrap();
            assert_eq!(
                observation.session_id.is_some(),
                identities.get("session_id").is_some()
            );
            assert_eq!(
                observation.turn_id.is_some(),
                identities.get("turn_id").is_some()
            );
        }
    }
}
