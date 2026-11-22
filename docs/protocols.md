# Protocol roadmap

Zim aims to support every public chat system through protocol adapters. Each adapter will document which account types, login flows, and messaging features it supports. A bridge, business integration, or bot can be useful when it is the available path. Its account type and limitations need to be visible in setup and in the route picker.

Use the [network issue backlog](https://github.com/unicornops/zim/labels/network-adapter) to track each network independently. Every adapter is a reusable Rust library with a headless consumer example. Voice calls, video calls, and screen sharing are part of the support target when the network offers them. See the [architecture](architecture.md) for the crate boundaries and call contract.

## First adapters

| System | Implementation | Discovery and release criteria |
| --- | --- | --- |
| Demo | In-process Rust adapter with two simulated networks | Deterministic presence, sends, and replies. Simulate replay after reconnect, unsupported presence, and uncertain delivery. |
| XMPP | Evaluate the Rust `xmpp-rs` stack against XMPP core and instant messaging specifications. [XMPP RFCs](https://xmpp.org/rfcs/) · [Rust XMPP docs](https://docs.xmpp.rs/main/tokio_xmpp/) | Direct text, roster, presence subscriptions, reconnect, and deduplication. Verify TLS and account behavior on a controlled server. Track OMEMO, message archives, and offline delivery coverage. |
| Matrix | Integrate the Matrix Rust SDK and use its encryption and sync implementation. [Matrix Rust SDK](https://github.com/matrix-org/matrix-rust-sdk) · [Client-server specification](https://spec.matrix.org/latest/client-server-api/) | Direct-room text, sync across restarts, end-to-end encryption, and device verification. Verify what each homeserver provides for presence. Exercise both fresh presence and unknown presence in routing tests. |

## Expansion backlog

| System | Candidate path and questions |
| --- | --- |
| IRC | Use IRCv3 specs and evaluate Rust libraries. Start with private messages over TLS, then measure support for presence, stable account IDs, and history across servers. [IRCv3 specifications](https://ircv3.net/irc/) |
| Telegram | Use the documented client API. Compare a Rust MTProto implementation with the official TDLib wrapped behind a narrow Rust interface. Evaluate login, terms, session storage, and chat types separately. [Telegram client registration](https://core.telegram.org/api/obtaining_api_id) · [TDLib](https://core.telegram.org/tdlib/getting-started) · [Rust candidate](https://github.com/Lonami/grammers) |
| Slack and Microsoft Teams | Evaluate Slack's Conversations API and Microsoft Graph with the intended user scopes, workspace or tenant approval, and the requirements for receiving updates. Record which chat actions and account types each integration can support. [Slack Conversations API](https://docs.slack.dev/apis/web-api/using-the-conversations-api/) · [Teams chat send](https://learn.microsoft.com/en-us/graph/api/chat-post-messages?view=graph-rest-1.0) · [Teams change notifications](https://learn.microsoft.com/en-us/graph/teams-changenotifications-chatmessage) |
| Discord | Evaluate supported bot or app workflows and their fit for individual chats. Discord prohibits automation of regular user accounts. [Discord account automation policy](https://support.discord.com/hc/en-us/articles/115002192352-Automated-User-Accounts-Self-Bots) |
| WhatsApp | Assess the Business Platform as a distinct adapter and investigate permitted interoperability paths for personal accounts. [WhatsApp Business Platform](https://developers.facebook.com/documentation/business-messaging/whatsapp/overview/) |
| Signal | Assess a supported path for personal accounts, linked-device behavior, upstream licensing, and ongoing protocol compatibility. Upstream describes use of `libsignal` outside Signal as unsupported. [libsignal](https://github.com/signalapp/libsignal) |
| iMessage, SMS, and RCS | Evaluate supported platform integrations and user-authorized device handoffs. Track OS, region, and account restrictions separately. Apple's Messages framework supports iMessage extensions, and Google's documented RCS for Business path is for branded agents. [Apple Messages framework](https://developer.apple.com/documentation/messages) · [Google RCS for Business](https://developers.google.com/business-communications/rcs-business-messaging/guides/get-started/how-it-works) |
| Further discovery | Inventory Google Chat, Messenger, Instagram, WeChat, LINE, Viber, KakaoTalk, Mattermost, Rocket.Chat, Zulip, and community-requested services. Rank by demand, feasibility for regular users, and the cost of maintaining compatibility. |

## How to add a protocol

1. Start with the service's documented client protocol or API and confirm supported account types, client registration, and scopes. Track direct-client support and bridge support separately. For restricted services, record the constraint and the remaining interoperability questions.
2. Prefer a maintained Rust SDK if it meets the feature and maintenance requirements. If the suitable library is written in another language, plan a focused Rust port where practical. For a licensed port, preserve upstream notices and verify its behavior against compatibility fixtures. Where a port would duplicate sensitive protocol or cryptographic code or delay support, use a maintained library through a narrow FFI or process boundary and review it over time.
3. For undocumented protocols, assess whether independent interoperability research and a third-party client are feasible for the service. Where permitted, document observed behavior using authorized test accounts and environments, publish a clean-room protocol description with synthetic fixtures, and implement against that description. Track upstream changes, maintenance ownership, and what would cause the adapter to be paused.
4. Implement the adapter as an independently consumable Rust library and test its public API from a headless Rust example. Document runtime, native dependencies, and any optional media features. Run the shared adapter contract tests for reconnect and replay, account isolation, send outcomes, and feature reporting. Check service-specific behavior on dedicated test accounts.
5. Record coverage for direct messages, presence, offline delivery, groups, attachments, and encryption. Evaluate voice calling, video calling, and screen sharing separately as described below. Update the network issue with verified capabilities and known gaps, and keep those states visible to routing and the UI.

## Calling and screen-sharing checklist

- For each network, record whether voice calls, video calls, and screen sharing are offered and which account types and integration routes expose them. Include direct and group coverage, initiator and recipient roles, supported platforms, and any required entitlements or deployment settings.
- Track each feature as verified, planned, blocked by the integration, unavailable on the network, or unknown. Document the provider or account constraint when the first-party client offers a feature that the integration cannot access. Use the same network issue when recording the gap.
- Verify incoming and outgoing calls separately. For screen sharing, verify sending and receiving, source selection and stop-share behavior, and screen audio where offered. Verify negotiated media security and cleanup on disconnect, hangup, permission denial, and revocation.
- Implement each available capability within the reusable adapter and shared host interfaces. A network can ship usable text while media work is tracked. Feature claims should reflect the account routes and platforms actually verified.
