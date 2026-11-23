//! Workspace-scoped Slack messaging through the documented Web and Events APIs.
//!
//! The host owns OAuth, credential storage, event authentication and delivery,
//! cache state, and retries. Run the asynchronous Web API methods on Tokio.

mod client;
mod events;

use std::time::Duration;

pub use client::SlackClient;

/// A verified Slack user installation in a workspace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AccountId {
    workspace_id: String,
    user_id: String,
}

impl AccountId {
    /// The workspace ID returned by `auth.test`.
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The user ID returned by `auth.test`.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Restore a conversation ID from storage scoped to this account.
    pub fn conversation(&self, id: impl Into<String>) -> ConversationId {
        ConversationId {
            account: self.clone(),
            id: id.into(),
        }
    }
}

/// A Slack conversation bound to the user and workspace that can access it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ConversationId {
    account: AccountId,
    id: String,
}

impl ConversationId {
    /// The authorized user and workspace for the conversation.
    pub fn account(&self) -> &AccountId {
        &self.account
    }

    /// The Slack conversation ID. Persist it with its account.
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

/// The kind of conversation to discover.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationKind {
    /// One-to-one direct messages.
    Direct,
    /// Group direct messages.
    GroupDirect,
    /// Public channels the user belongs to.
    PublicChannel,
    /// Private channels the user belongs to.
    PrivateChannel,
}

/// An accessible Slack conversation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversation {
    /// The account-scoped destination.
    pub id: ConversationId,
    /// The conversation type used for discovery.
    pub kind: ConversationKind,
    /// The channel name, when supplied by Slack.
    pub name: Option<String>,
    /// The other participant's Slack user ID for a one-to-one DM.
    pub direct_peer_id: Option<String>,
}

/// A provider page. Pass its cursor to the same operation to continue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page<T> {
    /// Records in Slack's order.
    pub items: Vec<T>,
    /// The opaque next cursor, if Slack reports another page.
    pub next_cursor: Option<String>,
}

/// A message ID scoped to the account and conversation that observed it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MessageId {
    /// Conversation containing the message.
    pub conversation: ConversationId,
    /// Slack's `ts` string. Preserve it verbatim for history and replies.
    pub timestamp: String,
}

/// A normalized Slack message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    /// Stable ID for reconciling history with incoming events.
    pub id: MessageId,
    /// Slack user ID for the author, when present.
    pub author_id: Option<String>,
    /// Text in Slack's original markup, when present.
    pub text: Option<String>,
    /// The root message's exact `thread_ts` string, if supplied by Slack.
    pub thread_timestamp: Option<String>,
}

/// A stable replay key from an Events API callback, scoped to the installation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EventId {
    /// The account selected by the host for this callback.
    pub account: AccountId,
    /// Slack's outer envelope `event_id` value.
    pub event_id: String,
}

/// A normalized Events API message change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageEvent {
    /// Stable key for deduplicating callbacks before applying the change.
    pub id: EventId,
    /// The operation and normalized message data.
    pub change: MessageChange,
}

/// A creation, edit, or deletion of a Slack message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageChange {
    /// A new message.
    Posted(Message),
    /// The new form of an existing message.
    Edited(Message),
    /// The ID of the deleted message.
    Deleted(MessageId),
}

/// Slack API or event errors. Treat an ambiguous send as pending reconciliation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The supplied bearer token could not be encoded.
    #[error("invalid Slack bearer token")]
    InvalidToken,
    /// A local request argument is missing or invalid.
    #[error("invalid Slack request input")]
    InvalidInput,
    /// The HTTPS transport could not be initialized.
    #[error("could not initialize the Slack HTTP client")]
    ClientSetup,
    /// The token or event belongs to another workspace.
    #[error("Slack workspace does not match the current account")]
    WorkspaceMismatch,
    /// The referenced conversation belongs to another user installation.
    #[error("Slack conversation does not belong to the current account")]
    AccountMismatch,
    /// A workspace-scoped user token is required.
    #[error("a Slack workspace user token is required")]
    UnsupportedAccount,
    /// Slack rejected the credentials or the grant has expired.
    #[error("Slack authentication is required")]
    AuthenticationRequired,
    /// The installation lacks a permission or membership required by Slack.
    #[error("Slack permission is required for this operation")]
    PermissionRequired,
    /// Slack does not support this operation through the current API.
    #[error("Slack does not support this operation")]
    UnsupportedOperation,
    /// Slack throttled the operation before accepting it.
    #[error("Slack rate limit reached")]
    RateLimited {
        /// Slack's `Retry-After` duration, when available.
        retry_after: Option<Duration>,
    },
    /// A transient transport or service failure occurred on a read.
    #[error("Slack is temporarily unavailable")]
    TemporaryFailure,
    /// Slack explicitly rejected the operation.
    #[error("Slack rejected the operation")]
    Rejected,
    /// Slack's payload did not match the documented API or event shape.
    #[error("unexpected Slack payload")]
    UnexpectedPayload,
    /// Slack may have created the message. Reconcile before sending again.
    #[error("Slack send outcome is unknown")]
    SendOutcomeUnknown,
}
