# Zim project plan

## First release

Deliver a GPUI desktop app for macOS, Windows, and Linux with an account setup screen, searchable people list, conversation timeline, and composer with a visible route selector. Use [UnicornOps’ Zmail](https://github.com/unicornops/zmail) as the cross-platform desktop reference. The [development and release guide](development.md) records Zim's build and packaging conventions.

The release test uses one person with an XMPP identity and a Matrix identity. Link them manually, send over XMPP while it is available, and then send the next message over Matrix when XMPP becomes unavailable and Matrix reports fresh availability. Test the same transition with Matrix presence unavailable and explicitly choose the Matrix route. Replies from both identities appear under the same person with their source network visible. Repeat with a pinned route. Restart the app and verify that the identity links, timeline, and pending send state survive. Run the desktop smoke test on all three target operating systems.

Start with direct text chats. Model group conversations as separate, network-owned threads so group routing can be designed later. Attachments, calls, group chat features, and third-party plugin installation follow the first release.

## Contacts and conversations

| Record | Purpose |
| --- | --- |
| `Person` | Stable local ID, display name, and route preferences. The people list uses these records. |
| `LocalAccount` | The signed-in identity used to send and receive on one service and server or tenant. Owns its session, sync state, and credential reference. |
| `ContactIdentity` | Provider-scoped identifier for somebody else, keyed by adapter, service instance, and remote user ID. A person can have multiple identities, including several on the same network. |
| `Route` | A local account, recipient identity, and provider conversation reference where required. Several routes can reach the same identity through different local accounts. |
| `Message` | Original route, provider conversation and message IDs, sender, timestamps, delivery status, and the encryption state reported for that message. One direct timeline displays messages across the person's linked identities. |

Users link and unlink identities explicitly. Linking can group existing histories, and unlinking restores the affected messages to the corresponding identity's own person record. Preserve message provenance and pending-send destinations throughout. Provider handles, display names, and profile photos can inform the UI, while identity membership comes from the user's choice. Adapters normalize identifiers according to their provider's rules and preserve server or tenant boundaries.

## Direct-message routing

1. Build eligible routes from linked identities and connected local accounts. Require support for direct text, a valid provider destination, and the active encryption requirement. Carry availability as `available`, `unavailable`, or `unknown`, with an observation time where supplied. Expired or missing presence becomes `unknown`.
2. Honor a one-message override or a persistent pin. If a pinned route cannot send, show its state and let the user choose another route. For automatic routing, prefer fresh availability, then use the person's route preference and most recent successful route to break ties. If every eligible route has unknown or unavailable presence, show the state and let the user explicitly choose a sendable route, including offline delivery where supported.
3. Show the local account, destination, service, and encryption state in the composer. If the route changes while the user is composing, retain the draft and show the update before dispatch. If the replacement loses end-to-end encryption, require a separate explicit choice where policy allows it. Block the send when the conversation requires encryption and no eligible encrypted route exists.
4. On send, persist the resolved route and a client message ID before handing the message to the adapter. Future messages can choose another network. If a submitted message times out or delivery is uncertain, keep it on its original route and reconcile by provider transaction ID or sync. Retry on that route only when it is known to be safe. If the adapter confirms no submission took place, the user can choose a new route for that message.

Keep presence and send eligibility distinct. A connected adapter may know a service accepts offline delivery even when it has no trustworthy visibility into whether the recipient is online. It only reports facts the service exposes.

## Rust workspace and adapter contract

| Boundary | First implementation |
| --- | --- |
| `zim-core` | Rust domain types, contact linking, route selection, conversation projection, and outbox rules. No GPUI or provider SDK dependencies. |
| `zim-protocol` | Async adapter contract and capability types. Include account setup, connect/disconnect, sync cursors, roster, presence, incoming messages, and send outcomes. Preserve provider identifiers and distinguish unsupported operations from temporary errors. |
| `zim-store` | SQLite for profiles, routes, messages, sync state, and the outbox. Commit incoming records and cursor updates together. Scope remote deduplication keys by local account and provider conversation. Store tokens in OS credential storage and use SDK-managed storage for encryption keys. Decide local message cache and retention policy before shipping. |
| `zim-desktop` | GPUI people list, direct timeline, composer, route controls, and account setup. Keep network work off the UI thread and use platform credential, notification, and packaging integrations. |
| `zim-adapter-*` | Demo, XMPP, and Matrix adapters behind the same contract. Each owns provider auth, feature discovery, encoding, reconnect behavior, and translation to core events. |

Start with separately compiled Rust crates registered in the app at build time. Adding an adapter should require its crate and registration, with reusable account and capability UI. Keep event delivery bounded and reconnects scoped to the affected account. After validating the contract on two real services, design a versioned out-of-process plugin interface. Give plugins per-account access, explicit capabilities, and failure isolation, and keep unrestricted third-party code out of the desktop process.

## Implementation order

| Order | Work | Done when |
| --- | --- | --- |
| 1 | Bootstrap Rust and GPUI | Commit a lockfile, pin upstream GPUI and Rust, and build the app shell with its native quit shortcut. Run linting, tests, and GPUI builds on all three target OSes. Set up native package automation and adopt Zmail's GPL-3.0-only license. |
| 2 | Model people, routes, and storage | Add persistent accounts, identity linking and unlinking, direct timeline projection, routing, and outbox state. Test same handle on separate services, stale presence, pins, encryption requirements, restart, and ambiguous send outcomes. |
| 3 | Prove the UI with a demo adapter | Simulate two independent services with controllable presence, disconnections, and delivery outcomes. Test one-person routing, incoming replies, and keyboard operation end to end without live accounts. |
| 4 | Add XMPP | Show login, roster, subscribed presence, direct text, reconnect, and incoming message deduplication on a controlled XMPP deployment. Surface encryption capability and track OMEMO scope separately. |
| 5 | Add Matrix and run the release test | Use the Matrix Rust SDK for sync and end-to-end encryption. Verify encrypted direct chats and device verification on test homeservers. Exercise the shared XMPP/Matrix timeline and routing with absent Matrix presence as well as a confirmed reachable destination. Verify credential storage, platform smoke tests, and trace redaction before packaging. |

Track plugin expansion and candidate protocol support in the [protocol roadmap](protocols.md).
