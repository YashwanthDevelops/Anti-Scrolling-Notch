//! The verified Codex CLI 0.157.1 observing-hook boundary.
//!
//! The disposable loopback probe observed four event names. This module accepts
//! only those names and intentionally discards every other input field (in
//! particular prompt text, paths, IDs, and tool payloads). It does not claim a
//! complete Codex hook schema or support for other Codex surfaces.

use serde::{Deserialize, Serialize};

pub const WIRE_VERSION: u8 = 1;
pub const MAX_HOOK_INPUT_BYTES: usize = 1 << 20;
pub const MAX_WIRE_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CodexHookEvent {
    #[serde(rename = "SessionStart")]
    SessionStart,
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit,
    #[serde(rename = "Stop")]
    Stop,
    #[serde(rename = "SessionEnd")]
    SessionEnd,
}

impl CodexHookEvent {
    pub const VERIFIED: [Self; 4] = [
        Self::SessionStart,
        Self::UserPromptSubmit,
        Self::Stop,
        Self::SessionEnd,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::Stop => "Stop",
            Self::SessionEnd => "SessionEnd",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "SessionStart" => Some(Self::SessionStart),
            "UserPromptSubmit" => Some(Self::UserPromptSubmit),
            "Stop" => Some(Self::Stop),
            "SessionEnd" => Some(Self::SessionEnd),
            _ => None,
        }
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    TooLarge,
    InvalidJson,
    NotObject,
    MissingEventName,
    UnsupportedEvent,
    UnsupportedWireVersion,
    InvalidWireMessage,
}

/// Convert stdin from Codex into a minimal, versioned internal observation.
/// The only upstream input member interpreted is the event name that the
/// sanitized recorder captured. All other JSON members are deliberately ignored.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
