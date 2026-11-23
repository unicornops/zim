use serde::Deserialize;
use serde_json::Value;

use crate::client::SlackMessage;
use crate::{Error, EventId, MessageChange, MessageEvent, MessageId, SlackClient};

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: String,
    team_id: Option<String>,
    event_id: Option<String>,
    event: Option<Value>,
}

impl SlackClient {
    /// Normalize a message callback already authenticated and routed by the host.
    ///
    /// The host verifies the Events API request signature or authenticated
    /// Socket Mode transport and selects an authorized installation before
    /// calling this method. This check applies to the workspace. The host also
    /// owns event acknowledgments and persistence of replay keys.
    ///
    /// For Socket Mode, pass the inner Events API payload. Non-message events
    /// and message subtypes outside the supported set return `Ok(None)`.
    pub fn normalize_authenticated_event(
        &self,
        body: &[u8],
    ) -> Result<Option<MessageEvent>, Error> {
        let envelope: Envelope =
            serde_json::from_slice(body).map_err(|_| Error::UnexpectedPayload)?;
        if envelope.kind != "event_callback" {
            return Ok(None);
        }
        let workspace = envelope.team_id.ok_or(Error::UnexpectedPayload)?;
        if workspace != self.account().workspace_id() {
            return Err(Error::WorkspaceMismatch);
        }

        let event = envelope.event.ok_or(Error::UnexpectedPayload)?;
        if required_string(&event, "type")? != "message" {
            return Ok(None);
        }
        let account = self.account();
        let subtype = event
            .get("subtype")
            .map(|subtype| subtype.as_str().ok_or(Error::UnexpectedPayload))
            .transpose()?;
        let change = match subtype {
            None | Some("bot_message" | "thread_broadcast") => {
                let conversation = account.conversation(required_string(&event, "channel")?);
                let message: SlackMessage =
                    serde_json::from_value(event).map_err(|_| Error::UnexpectedPayload)?;
                MessageChange::Posted(message.normalize(&conversation)?)
            }
            Some("message_changed") => {
                let conversation = account.conversation(required_string(&event, "channel")?);
                let message: SlackMessage = serde_json::from_value(event["message"].clone())
                    .map_err(|_| Error::UnexpectedPayload)?;
                MessageChange::Edited(message.normalize(&conversation)?)
            }
            Some("message_deleted") => MessageChange::Deleted(MessageId {
                conversation: account.conversation(required_string(&event, "channel")?),
                timestamp: required_string(&event, "deleted_ts")?.into(),
            }),
            Some(_) => return Ok(None),
        };
        let event_id = envelope
            .event_id
            .filter(|id| !id.is_empty())
            .ok_or(Error::UnexpectedPayload)?;
        Ok(Some(MessageEvent {
            id: EventId {
                account: account.clone(),
                event_id,
            },
            change,
        }))
    }
}

fn required_string<'a>(payload: &'a Value, key: &str) -> Result<&'a str, Error> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(Error::UnexpectedPayload)
}
