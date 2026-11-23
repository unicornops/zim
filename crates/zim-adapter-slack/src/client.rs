use std::time::Duration;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{
    AccountId, Conversation, ConversationId, ConversationKind, Error, Message, MessageId, Page,
};

const API_ORIGIN: &str = "https://slack.com/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A client bound to a verified Slack user and workspace.
#[derive(Clone)]
pub struct SlackClient {
    http: Client,
    api_origin: String,
    account: AccountId,
}

impl SlackClient {
    /// Verify a caller-supplied Slack user token with `auth.test`.
    ///
    /// The caller obtains and refreshes the grant. Bot and organization-wide
    /// installations without a workspace-bound user are unsupported.
    pub async fn connect(token: &str, expected_workspace: &str) -> Result<Self, Error> {
        Self::connect_to(token, expected_workspace, API_ORIGIN).await
    }

    async fn connect_to(
        token: &str,
        expected_workspace: &str,
        api_origin: &str,
    ) -> Result<Self, Error> {
        if expected_workspace.is_empty() {
            return Err(Error::InvalidInput);
        }
        if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(Error::InvalidToken);
        }
        let mut bearer =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Error::InvalidToken)?;
        bearer.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, bearer);
        let http = Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| Error::ClientSetup)?;

        let identity: Identity = call(
            http.post(format!("{api_origin}/auth.test")),
            Operation::Read,
        )
        .await?;
        if identity.bot_id.is_some() {
            return Err(Error::UnsupportedAccount);
        }
        let Some(workspace_id) = identity.team_id else {
            return Err(Error::UnsupportedAccount);
        };
        if workspace_id != expected_workspace {
            return Err(Error::WorkspaceMismatch);
        }
        let user_id = identity
            .user_id
            .filter(|id| !id.is_empty())
            .ok_or(Error::UnexpectedPayload)?;

        Ok(Self {
            http,
            api_origin: api_origin.into(),
            account: AccountId {
                workspace_id,
                user_id,
            },
        })
    }

    /// The user and workspace verified for this token.
    pub fn account(&self) -> &AccountId {
        &self.account
    }

    /// Discover a page of conversations of the requested type with `users.conversations`.
    pub async fn conversations(
        &self,
        kind: ConversationKind,
        cursor: Option<&str>,
    ) -> Result<Page<Conversation>, Error> {
        let kind_parameter = match kind {
            ConversationKind::Direct => "im",
            ConversationKind::GroupDirect => "mpim",
            ConversationKind::PublicChannel => "public_channel",
            ConversationKind::PrivateChannel => "private_channel",
        };
        let request = self
            .http
            .get(self.endpoint("users.conversations"))
            .query(&[("types", kind_parameter), ("exclude_archived", "true")]);
        let response: ConversationPage =
            call(with_cursor(request, cursor), Operation::Read).await?;
        let items = response
            .channels
            .into_iter()
            .map(|channel| self.conversation(kind, channel))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            next_cursor: next_cursor(response.response_metadata),
        })
    }

    /// Open or resume a one-to-one DM with the supplied Slack user ID.
    pub async fn open_direct(&self, peer_id: &str) -> Result<Conversation, Error> {
        if peer_id.is_empty() || peer_id.contains(',') {
            return Err(Error::InvalidInput);
        }
        let request = self
            .http
            .post(self.endpoint("conversations.open"))
            .json(&json!({"users": peer_id}));
        let response: OpenConversation = call(request, Operation::Read).await?;
        let mut channel = response.channel;
        channel.user = Some(peer_id.into());
        self.conversation(ConversationKind::Direct, channel)
    }

    /// Send text to a conversation or as a reply to a thread root.
    ///
    /// Slack interprets its message markup. Automatic link and media previews
    /// are disabled for this operation.
    /// Reconcile a [`Error::SendOutcomeUnknown`] with Slack before retrying.
    pub async fn send_text(
        &self,
        conversation: &ConversationId,
        text: &str,
        thread_root: Option<&MessageId>,
    ) -> Result<MessageId, Error> {
        self.validate_conversation(conversation)?;
        if text.is_empty() {
            return Err(Error::InvalidInput);
        }
        let mut body = json!({
            "channel": conversation.as_str(),
            "text": text,
            "unfurl_links": false,
            "unfurl_media": false,
        });
        if let Some(root) = thread_root {
            self.validate_conversation(&root.conversation)?;
            if root.conversation != *conversation || root.timestamp.is_empty() {
                return Err(Error::InvalidInput);
            }
            body["thread_ts"] = json!(root.timestamp);
        }
        let request = self
            .http
            .post(self.endpoint("chat.postMessage"))
            .json(&body);
        let response: SentMessage = call(request, Operation::Send).await?;
        if response.channel != conversation.as_str() || response.ts.is_empty() {
            return Err(Error::SendOutcomeUnknown);
        }
        Ok(MessageId {
            conversation: conversation.clone(),
            timestamp: response.ts,
        })
    }

    /// Retrieve a page of conversation history in Slack's order.
    pub async fn history(
        &self,
        conversation: &ConversationId,
        cursor: Option<&str>,
    ) -> Result<Page<Message>, Error> {
        self.validate_conversation(conversation)?;
        let request = self
            .http
            .get(self.endpoint("conversations.history"))
            .query(&[("channel", conversation.as_str())]);
        Self::message_page(request, conversation, cursor).await
    }

    /// Retrieve a page of a thread. Slack may include the root in the response.
    pub async fn replies(
        &self,
        root: &MessageId,
        cursor: Option<&str>,
    ) -> Result<Page<Message>, Error> {
        self.validate_conversation(&root.conversation)?;
        if root.timestamp.is_empty() {
            return Err(Error::InvalidInput);
        }
        let request = self
            .http
            .get(self.endpoint("conversations.replies"))
            .query(&[
                ("channel", root.conversation.as_str()),
                ("ts", &root.timestamp),
            ]);
        Self::message_page(request, &root.conversation, cursor).await
    }

    fn endpoint(&self, method: &str) -> String {
        format!("{}/{method}", self.api_origin)
    }

    fn validate_conversation(&self, conversation: &ConversationId) -> Result<(), Error> {
        if conversation.account() != &self.account {
            return Err(Error::AccountMismatch);
        }
        if conversation.as_str().is_empty() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    fn conversation(
        &self,
        kind: ConversationKind,
        channel: Channel,
    ) -> Result<Conversation, Error> {
        if channel.id.is_empty() {
            return Err(Error::UnexpectedPayload);
        }
        Ok(Conversation {
            id: self.account.conversation(channel.id),
            kind,
            name: channel.name,
            direct_peer_id: channel.user,
        })
    }

    async fn message_page(
        request: RequestBuilder,
        conversation: &ConversationId,
        cursor: Option<&str>,
    ) -> Result<Page<Message>, Error> {
        let response: MessagePage = call(with_cursor(request, cursor), Operation::Read).await?;
        let items = response
            .messages
            .into_iter()
            .map(|message| message.normalize(conversation))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            next_cursor: next_cursor(response.response_metadata),
        })
    }
}

#[derive(Deserialize)]
struct Identity {
    team_id: Option<String>,
    user_id: Option<String>,
    bot_id: Option<String>,
}

#[derive(Deserialize)]
struct Channel {
    id: String,
    name: Option<String>,
    user: Option<String>,
}

#[derive(Deserialize)]
struct OpenConversation {
    channel: Channel,
}

#[derive(Deserialize)]
struct ConversationPage {
    channels: Vec<Channel>,
    response_metadata: Option<Cursor>,
}

#[derive(Deserialize)]
struct SentMessage {
    channel: String,
    ts: String,
}

#[derive(Deserialize)]
struct MessagePage {
    messages: Vec<SlackMessage>,
    response_metadata: Option<Cursor>,
}

#[derive(Deserialize)]
struct Cursor {
    next_cursor: Option<String>,
}

fn next_cursor(cursor: Option<Cursor>) -> Option<String> {
    cursor
        .and_then(|cursor| cursor.next_cursor)
        .filter(|cursor| !cursor.is_empty())
}

fn with_cursor(request: RequestBuilder, cursor: Option<&str>) -> RequestBuilder {
    match cursor {
        Some(cursor) => request.query(&[("cursor", cursor)]),
        None => request,
    }
}

#[derive(Deserialize)]
pub(super) struct SlackMessage {
    ts: String,
    user: Option<String>,
    text: Option<String>,
    thread_ts: Option<String>,
}

impl SlackMessage {
    pub(super) fn normalize(self, conversation: &ConversationId) -> Result<Message, Error> {
        if self.ts.is_empty() {
            return Err(Error::UnexpectedPayload);
        }
        Ok(Message {
            id: MessageId {
                conversation: conversation.clone(),
                timestamp: self.ts,
            },
            author_id: self.user,
            text: self.text,
            thread_timestamp: self.thread_ts,
        })
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Read,
    Send,
}

impl Operation {
    fn failure(self, error: Error) -> Error {
        match self {
            Self::Read => error,
            Self::Send => Error::SendOutcomeUnknown,
        }
    }
}

async fn call<T: DeserializeOwned>(
    request: RequestBuilder,
    operation: Operation,
) -> Result<T, Error> {
    let response = request
        .send()
        .await
        .map_err(|_| operation.failure(Error::TemporaryFailure))?;
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|header| header.to_str().ok())
                .and_then(|value| value.parse().ok())
                .map(Duration::from_secs);
            return Err(Error::RateLimited { retry_after });
        }
        StatusCode::UNAUTHORIZED => return Err(Error::AuthenticationRequired),
        StatusCode::FORBIDDEN => return Err(Error::PermissionRequired),
        StatusCode::REQUEST_TIMEOUT => return Err(operation.failure(Error::TemporaryFailure)),
        status if status.is_server_error() => {
            return Err(operation.failure(Error::TemporaryFailure));
        }
        status if status.is_client_error() => return Err(Error::Rejected),
        status if !status.is_success() => return Err(operation.failure(Error::UnexpectedPayload)),
        _ => {}
    }

    let payload: Value = response.json().await.map_err(|error| {
        let error = if error.is_decode() {
            Error::UnexpectedPayload
        } else {
            Error::TemporaryFailure
        };
        operation.failure(error)
    })?;
    match payload.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            serde_json::from_value(payload).map_err(|_| operation.failure(Error::UnexpectedPayload))
        }
        Some(false) => {
            let code = payload
                .get("error")
                .and_then(Value::as_str)
                .ok_or_else(|| operation.failure(Error::UnexpectedPayload))?;
            Err(slack_error(code, operation))
        }
        None => Err(operation.failure(Error::UnexpectedPayload)),
    }
}

fn slack_error(code: &str, operation: Operation) -> Error {
    match code {
        "not_authed" | "invalid_auth" | "token_revoked" | "token_expired" | "account_inactive" => {
            Error::AuthenticationRequired
        }
        "missing_scope" | "no_permission" | "not_in_channel" | "restricted_action" => {
            Error::PermissionRequired
        }
        "unknown_method" | "method_deprecated" | "not_supported" | "not_allowed_token_type" => {
            Error::UnsupportedOperation
        }
        "ratelimited" => Error::RateLimited { retry_after: None },
        "internal_error"
        | "fatal_error"
        | "service_unavailable"
        | "temporarily_unavailable"
        | "request_timeout" => operation.failure(Error::TemporaryFailure),
        _ => Error::Rejected,
    }
}

#[cfg(test)]
mod tests;
