# Zim Slack adapter

This Rust library provides documented Slack Web API messaging and Events API message normalization. It runs on Tokio without Zim's UI, store, or application crates. The internal HTTPS transport uses reqwest, ring-backed Rustls, and native root certificates. Slack Morphism is a broader SDK that includes event transports and other Slack app features. This adapter uses direct requests for its small documented surface and exposes account-scoped Rust types. [Slack Morphism](https://github.com/abdolence/slack-morphism-rust)

## Account and permissions

Obtain a workspace-scoped user access token using an authorized Slack app and the host's OAuth flow. Slack supports OAuth PKCE for desktop public clients. The host owns authorization and redirect handling, storage, refresh, and revocation. Pass the chosen workspace ID to `SlackClient::connect`. The adapter confirms the token's workspace and user with `auth.test`, rejects bot accounts and restricts its HTTP transport to Slack's HTTPS API. When restoring an account, compare the verified user ID with the saved account before loading cached state. Use an approved test workspace for live verification. [Slack PKCE](https://docs.slack.dev/authentication/using-pkce/) · [Slack auth.test](https://docs.slack.dev/reference/methods/auth.test/)

| Operation | Documented Slack user scopes |
| --- | --- |
| Discover conversations | `im:read`, `mpim:read`, `channels:read`, or `groups:read` according to the requested conversation type. [users.conversations](https://docs.slack.dev/reference/methods/users.conversations/) |
| Open a one-to-one DM | `im:write`. [conversations.open](https://docs.slack.dev/reference/methods/conversations.open/) |
| Send a message or reply | `chat:write` under the user grant. Slack interprets its markup. The adapter disables link and media previews. [chat.postMessage](https://docs.slack.dev/reference/methods/chat.postMessage/) |
| Read a conversation, thread, or message events | The applicable `im:history`, `mpim:history`, `channels:history`, or `groups:history` scope. Event delivery also requires the corresponding event subscription. [conversations.history](https://docs.slack.dev/reference/methods/conversations.history/) · [conversations.replies](https://docs.slack.dev/reference/methods/conversations.replies/) · [Events API](https://docs.slack.dev/apis/events-api/) |

Availability depends on the user's membership, app installation, workspace policy, and Slack's distribution rules. Slack's public Calls API integrates calls hosted by an external service. Native Slack Huddle audio, camera video, and screen sharing remain separate feasibility work in [issue #11](https://github.com/unicornops/zim/issues/11). [Slack rate limits](https://docs.slack.dev/apis/web-api/rate-limits/) · [Slack Calls API](https://docs.slack.dev/apis/web-api/using-the-calls-api/)

## Event delivery and reconciliation

Verify incoming HTTP request signatures or use an authenticated Socket Mode connection. Select each authorized user installation before passing an Events API callback body to `normalize_authenticated_event`. For Socket Mode, pass the inner Events API payload. The adapter compares the callback workspace with the connected workspace. The host handles a callback's installation authorizations, including when Slack lists a single authorization for an event visible to multiple installations. The host also owns acknowledgments, buffering, and reconnect behavior. [Slack Events API](https://docs.slack.dev/apis/events-api/) · [Verifying requests](https://docs.slack.dev/authentication/verifying-requests-from-slack/) · [Socket Mode](https://docs.slack.dev/apis/events-api/using-socket-mode/)

Persist account IDs with conversation IDs and store provider timestamps as strings. Use `EventId` for event replay deduplication and `MessageId` to reconcile events with conversation or thread history. Reuse an opaque cursor only for the same account and operation. On disconnect or event gaps, fetch history and reconcile by message ID. The adapter performs no automatic retries. Honor `RateLimited` and its optional `Retry-After` value. For `SendOutcomeUnknown`, reconcile against Slack before sending the message again.

## Headless consumer

The example reads the first page of DMs with a test token and prints only its record count. Supply `ZIM_SLACK_TOKEN` through the host's environment and set `ZIM_SLACK_WORKSPACE` to the verified workspace ID.

```sh
cargo run -p zim-adapter-slack --example slack_discover
cargo test -p zim-adapter-slack --locked
```

Another Rust application can use this crate as a path dependency, or as a Git workspace dependency. CI exercises the adapter with local mock Slack responses and synthetic callbacks. Live workspace interoperability remains to be verified.
