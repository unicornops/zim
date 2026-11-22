# Zim contributor instructions

Zim is a Rust desktop client built with GPUI for macOS, Windows, and Linux. People are the top-level contacts. Each person can link multiple network identities, while messages and calls retain their original account and network. The [architecture](docs/architecture.md) defines the boundaries for application code, protocol adapters, and media. The [project plan](docs/plan.md) sets the implementation order.

## Adapter work

- Put each network in its own reusable Rust library crate, normally `crates/zim-adapter-<network>`. Keep it consumable by a separate Rust program without Zim's desktop, storage, or contact model. Depend on the shared adapter contract and keep provider SDK types and runtime details private to the adapter.
- Keep protocol rules in the adapter and person linking, routing, and encryption policy in the application. Keep GPUI code in `zim-desktop`. Provide credential access, session storage, configuration, and platform media services through narrow interfaces when the adapter needs them.
- Evaluate voice calls, video calls, and screen sharing independently for every network. Implement each feature that the supported account route can offer. Record the requirements and gaps for other account types, platforms, and API routes. Update the network issue and its capability documentation as support changes.
- Keep active calls pinned to the chosen account, provider, and call session. Announce the route before placing an outgoing call. Require explicit permission to capture a microphone, camera, or screen and stop capture when the session ends.
- Prefer documented service interfaces and maintained Rust implementations. Keep bindings or a separate process behind the adapter when an upstream SDK is a better fit. Record upstream versions, licenses, and compatibility limits. Use authorized accounts and synthetic fixtures for interoperability work.
- Add behavior tests with the adapter and use the shared contract tests as they become available. Include a headless example a consuming Rust app can run. Keep credentials, message text, call metadata that can identify users, and media out of fixtures and routine logs.

See the [protocol roadmap](docs/protocols.md) for the per-network checklist and the [network issue backlog](https://github.com/unicornops/zim/labels/network-adapter) for work to pick up.

## Verification and releases

Use the pinned toolchain in `rust-toolchain.toml`. Run these checks for code changes and check the workspace and package features affected by a new adapter.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Use Conventional Commits for changes that should appear in Release Please. Follow the [development and release guide](docs/development.md) for native dependencies, CI, packaging, and release dry runs. Zim uses [GPL-3.0-only](LICENSE).
