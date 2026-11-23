use std::time::Duration;

use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::MessageChange;

async fn start() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth.test"))
        .and(header("authorization", "Bearer synthetic-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"ok": true, "team_id": "T_LOCAL", "user_id": "U_LOCAL"})),
        )
        .mount(&server)
        .await;
    server
}

async fn connect(server: &MockServer) -> SlackClient {
    SlackClient::connect_to(
        "synthetic-token",
        "T_LOCAL",
        &format!("{}/api", server.uri()),
    )
    .await
    .expect("verify the synthetic Slack account")
}

fn callback(team: &str, event: Value) -> Vec<u8> {
    json!({"type": "event_callback", "team_id": team, "event_id": "Ev_SYNTHETIC", "event": event})
        .to_string()
        .into_bytes()
}

#[tokio::test]
async fn verifies_workspace_and_rejects_bots() {
    let server = start().await;
    let url = format!("{}/api", server.uri());
    let client = connect(&server).await;
    assert_eq!(client.account().workspace_id(), "T_LOCAL");
    assert_eq!(client.account().user_id(), "U_LOCAL");
    assert!(matches!(
        SlackClient::connect_to("synthetic-token", "T_OTHER", &url).await,
        Err(Error::WorkspaceMismatch),
    ));

    Mock::given(method("POST"))
        .and(path("/api/auth.test"))
        .and(header("authorization", "Bearer synthetic-bot-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"ok": true, "team_id": "T_LOCAL", "user_id": "U_BOT", "bot_id": "B_SYNTHETIC"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        SlackClient::connect_to("synthetic-bot-token", "T_LOCAL", &url).await,
        Err(Error::UnsupportedAccount),
    ));
}

#[tokio::test]
async fn discovers_and_opens_dms_and_pages_history_and_replies() {
    let server = start().await;
    let client = connect(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/users.conversations"))
        .and(header("authorization", "Bearer synthetic-token"))
        .and(query_param("types", "im"))
        .and(query_param("cursor", "=before"))
        .and(query_param("exclude_archived", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "channels": [{"id": "D_LOCAL", "user": "U_PEER"}],
            "response_metadata": {"next_cursor": "after="},
        })))
        .expect(1)
        .mount(&server)
        .await;
    let page = client
        .conversations(ConversationKind::Direct, Some("=before"))
        .await
        .expect("discover a synthetic DM");
    assert_eq!(page.next_cursor.as_deref(), Some("after="));
    let dm = &page.items[0];
    assert_eq!(dm.direct_peer_id.as_deref(), Some("U_PEER"));
    assert_eq!(dm.id.account(), client.account());

    Mock::given(method("POST"))
        .and(path("/api/conversations.open"))
        .and(body_json(json!({"users": "U_PEER"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"ok": true, "channel": {"id": "D_LOCAL"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let reopened = client.open_direct("U_PEER").await.expect("resume the DM");
    assert_eq!(&reopened, dm);

    Mock::given(method("GET"))
        .and(path("/api/conversations.history"))
        .and(query_param("channel", "D_LOCAL"))
        .and(query_param("cursor", "more=history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [{"ts": "1798021449.000001", "user": "U_PEER", "text": "Synthetic root"}],
            "response_metadata": {"next_cursor": ""},
        })))
        .expect(1)
        .mount(&server)
        .await;
    let messages = client
        .history(&dm.id, Some("more=history"))
        .await
        .expect("read a history page");
    assert_eq!(messages.next_cursor, None);
    let root = &messages.items[0].id;
    assert_eq!(root.timestamp, "1798021449.000001");

    Mock::given(method("GET"))
        .and(path("/api/conversations.replies"))
        .and(query_param("channel", "D_LOCAL"))
        .and(query_param("ts", "1798021449.000001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [{"ts": "1798021450.000002", "thread_ts": "1798021449.000001"}],
            "response_metadata": {"next_cursor": "replies="},
        })))
        .expect(1)
        .mount(&server)
        .await;
    let replies = client.replies(root, None).await.expect("read a reply page");
    assert_eq!(replies.next_cursor.as_deref(), Some("replies="));
    assert_eq!(
        replies.items[0].thread_timestamp.as_deref(),
        Some(root.timestamp.as_str())
    );

    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .and(body_json(json!({
            "channel": "D_LOCAL", "text": "Synthetic reply", "thread_ts": "1798021449.000001",
            "unfurl_links": false, "unfurl_media": false,
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"ok": true, "channel": "D_LOCAL", "ts": "1798021450.000011"}),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let sent = client
        .send_text(&dm.id, "Synthetic reply", Some(root))
        .await
        .expect("send a reply");
    assert_eq!(sent.conversation, dm.id);
    assert_eq!(sent.timestamp, "1798021450.000011");
}

#[tokio::test]
async fn scopes_ids_and_events_before_use() {
    let server = start().await;
    let client = connect(&server).await;
    Mock::given(path("/api/conversations.history"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth.test"))
        .and(header("authorization", "Bearer synthetic-other-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"ok": true, "team_id": "T_OTHER", "user_id": "U_LOCAL"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let other_workspace = SlackClient::connect_to(
        "synthetic-other-token",
        "T_OTHER",
        &format!("{}/api", server.uri()),
    )
    .await
    .expect("verify the other synthetic Slack workspace");
    let other_user = AccountId {
        workspace_id: "T_LOCAL".into(),
        user_id: "U_OTHER".into(),
    };
    for other in [other_workspace.account(), &other_user] {
        let foreign_dm = other.conversation("D_LOCAL");
        assert_ne!(foreign_dm, client.account().conversation("D_LOCAL"));
        assert!(matches!(
            client.history(&foreign_dm, None).await,
            Err(Error::AccountMismatch)
        ));
    }

    let posted = json!({"type": "message", "channel": "D_LOCAL", "user": "U_PEER", "ts": "1798021449.000001"});
    let foreign_event = callback("T_OTHER", posted.clone());
    assert_eq!(
        client.normalize_authenticated_event(&foreign_event),
        Err(Error::WorkspaceMismatch)
    );

    let event_bytes = callback("T_LOCAL", posted);
    let first = client
        .normalize_authenticated_event(&event_bytes)
        .expect("event parses")
        .expect("event is posted text");
    let replay = client
        .normalize_authenticated_event(&event_bytes)
        .expect("same event parses")
        .expect("same event is posted text");
    assert_eq!(first.id, replay.id);
    assert_eq!(first.id.account, *client.account());
    assert_eq!(first.id.event_id, "Ev_SYNTHETIC");
    let other_event = other_workspace
        .normalize_authenticated_event(&foreign_event)
        .expect("event parses in the other workspace")
        .expect("event is posted text");
    assert_ne!(first.id, other_event.id);
    match first.change {
        MessageChange::Posted(message) => {
            assert_eq!(message.id.timestamp, "1798021449.000001");
            assert_eq!(
                message.id.conversation,
                client.account().conversation("D_LOCAL")
            );
        }
        change => panic!("expected a posted message, got {change:?}"),
    }
}

#[tokio::test]
async fn normalizes_edits_and_deletes_and_ignores_other_events() {
    let server = start().await;
    let client = connect(&server).await;
    let edited = callback(
        "T_LOCAL",
        json!({"type": "message", "subtype": "message_changed", "channel": "D_LOCAL",
        "ts": "1798021450.000002", "message": {"ts": "1798021449.000001", "text": "Synthetic edit"}}),
    );
    let deleted = callback(
        "T_LOCAL",
        json!({"type": "message", "subtype": "message_deleted", "channel": "D_LOCAL",
        "ts": "1798021451.000003", "deleted_ts": "1798021449.000001"}),
    );
    let expected_id = MessageId {
        conversation: client.account().conversation("D_LOCAL"),
        timestamp: "1798021449.000001".into(),
    };
    assert!(matches!(client.normalize_authenticated_event(&edited),
        Ok(Some(event)) if matches!(&event.change, MessageChange::Edited(message) if message.id == expected_id)));
    assert!(matches!(client.normalize_authenticated_event(&deleted),
        Ok(Some(event)) if event.change == MessageChange::Deleted(expected_id)));
    let irrelevant = callback("T_LOCAL", json!({"type": "reaction_added"}));
    assert_eq!(client.normalize_authenticated_event(&irrelevant), Ok(None));
    let bad = callback(
        "T_LOCAL",
        json!({"type": "message", "channel": "D_LOCAL", "ts": 1798021449.000001}),
    );
    assert_eq!(
        client.normalize_authenticated_event(&bad),
        Err(Error::UnexpectedPayload)
    );
}

#[tokio::test]
async fn distinguishes_retry_after_definite_rejection_and_unknown_delivery() {
    let server = start().await;
    let client = connect(&server).await;
    let dm = client.account().conversation("D_LOCAL");
    Mock::given(method("GET"))
        .and(path("/api/conversations.history"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "17"))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        client.history(&dm, None).await,
        Err(Error::RateLimited {
            retry_after: Some(Duration::from_secs(17))
        })
    );

    for (text, response, expected) in [
        (
            "server",
            ResponseTemplate::new(503),
            Error::SendOutcomeUnknown,
        ),
        (
            "timeout",
            ResponseTemplate::new(408),
            Error::SendOutcomeUnknown,
        ),
        (
            "fatal",
            ResponseTemplate::new(200).set_body_json(json!({"ok": false, "error": "fatal_error"})),
            Error::SendOutcomeUnknown,
        ),
        (
            "scope",
            ResponseTemplate::new(200)
                .set_body_json(json!({"ok": false, "error": "missing_scope"})),
            Error::PermissionRequired,
        ),
        (
            "malformed",
            ResponseTemplate::new(200).set_body_json(json!({"ok": true, "channel": "D_LOCAL"})),
            Error::SendOutcomeUnknown,
        ),
    ] {
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .and(body_json(json!({"channel": "D_LOCAL", "text": text, "unfurl_links": false, "unfurl_media": false})))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(client.send_text(&dm, text, None).await, Err(expected));
    }
}

#[tokio::test]
async fn does_not_follow_redirects() {
    let server = MockServer::start().await;
    Mock::given(path("/api/auth.test"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", format!("{}/other", server.uri())),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/other"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    assert!(matches!(
        SlackClient::connect_to(
            "synthetic-token",
            "T_LOCAL",
            &format!("{}/api", server.uri())
        )
        .await,
        Err(Error::UnexpectedPayload),
    ));
}
