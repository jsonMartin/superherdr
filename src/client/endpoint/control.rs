use super::ClientEndpointId;

pub(crate) enum EndpointControlMessage {
    HealthPong,
    Snapshot(Box<crate::protocol::ClientShellSnapshot>),
    WorkspaceSnoozeState(Box<crate::api::schema::WorkspaceSnoozeState>),
    OptionalFeatureError { feature: String, error: String },
    Ignored,
}

pub(crate) fn decode_endpoint_control(
    kind: &str,
    data: &str,
) -> Result<EndpointControlMessage, String> {
    if kind == crate::protocol::endpoint::HEALTH_PONG_KIND {
        return Ok(EndpointControlMessage::HealthPong);
    }
    if kind == crate::protocol::endpoint::ENDPOINT_SNAPSHOT_KIND {
        let snapshot = serde_json::from_str(data)
            .map_err(|error| format!("invalid endpoint snapshot: {error}"))?;
        return Ok(EndpointControlMessage::Snapshot(Box::new(snapshot)));
    }
    if kind == "workspace_snooze.state" {
        return match serde_json::from_str(data) {
            Ok(state) => Ok(EndpointControlMessage::WorkspaceSnoozeState(Box::new(state))),
            Err(error) => Ok(EndpointControlMessage::OptionalFeatureError {
                feature: "workspace_snooze.state".into(),
                error: format!("invalid endpoint workspace snooze state: {error}"),
            }),
        };
    }
    if kind.starts_with("shell.snapshot.") {
        return Err(format!(
            "unsupported mandatory endpoint snapshot codec {kind:?}"
        ));
    }
    Ok(EndpointControlMessage::Ignored)
}

pub(crate) fn protocol_failure_is_fatal(endpoint_id: &ClientEndpointId) -> bool {
    endpoint_id.is_local()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::endpoint::ProfileId;

    #[test]
    fn unknown_optional_controls_are_ignored() {
        assert!(matches!(
            decode_endpoint_control("future.optional", "not json").unwrap(),
            EndpointControlMessage::Ignored
        ));
    }

    #[test]
    fn unknown_snapshot_codecs_are_rejected() {
        assert_eq!(
            decode_endpoint_control("shell.snapshot.v2", "{}")
                .err()
                .as_deref(),
            Some("unsupported mandatory endpoint snapshot codec \"shell.snapshot.v2\"")
        );
    }

    #[test]
    fn malformed_optional_snooze_state_returns_optional_feature_error_without_protocol_failure() {
        let message = decode_endpoint_control("workspace_snooze.state", "{not json}").unwrap();
        match message {
            EndpointControlMessage::OptionalFeatureError { feature, error } => {
                assert_eq!(feature, "workspace_snooze.state");
                assert!(error.contains("invalid endpoint workspace snooze state"));
            }
            _ => panic!("expected OptionalFeatureError"),
        }
    }

    #[test]
    fn decodes_workspace_snooze_state() {
        let json = r#"{"boot_id":"boot-1","revision":1,"records":[{"workspace_id":"ws_1","boot_id":"boot-1","deadline_unix_ms":1000,"revision":1}]}"#;
        let message = decode_endpoint_control("workspace_snooze.state", json).unwrap();
        match message {
            EndpointControlMessage::WorkspaceSnoozeState(state) => {
                assert_eq!(state.boot_id, "boot-1");
                assert_eq!(state.revision, 1);
                assert_eq!(state.records.len(), 1);
                assert_eq!(state.records[0].workspace_id, "ws_1");
            }
            _ => panic!("expected WorkspaceSnoozeState"),
        }
    }

    #[test]
    fn only_local_protocol_failures_end_the_client() {
        let remote =
            ClientEndpointId::Ssh(ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap());
        assert!(protocol_failure_is_fatal(&ClientEndpointId::Local));
        assert!(!protocol_failure_is_fatal(&remote));
    }
}
