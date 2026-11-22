# Development and releases

`zim-app` is the workspace-root package and native entry point. `crates/zim-desktop` owns the GPUI shell. The workspace pins `gpui` and `gpui_platform` to Zed's `v1.24.2` tag, the stable baseline previously tested by Zmail. Their resolved commit is in `Cargo.lock`. Update both dependencies and the `async-task` and `calloop` patches together when changing the pin.

## Build and test

Rustup selects the toolchain from `rust-toolchain.toml`. macOS uses Xcode's developer tools, and Windows uses the MSVC Rust target and Windows SDK. On Ubuntu 24.04, install the native build and graphics dependencies with these commands.

```sh
sudo apt-get update
sudo apt-get install -y pkg-config libdbus-1-dev libasound2-dev libxcb1-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libfontconfig1-dev libfreetype6-dev \
  libvulkan1 mesa-vulkan-drivers
```

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build -p zim-app --release --locked
cargo run --locked
```

CI runs on pull requests and pushes to `main`. Each desktop runner lints, tests, and builds the app. The GPUI test opens the real workspace with GPUI's headless platform. Linux CI also launches the native binary and looks for its visible window under Xvfb. With `xvfb` and `xdotool` installed, use `xvfb-run -a bash scripts/smoke-desktop.sh target/release/zim-app` to run that check locally. A separate job runs `cargo deny --locked check licenses advisories bans sources` and `cargo audit`.

## Publish a release

Use Conventional Commits for changes that should appear in the changelog. Release Please opens or updates a release PR from `main`. Its Rust strategy versions the workspace crates and lockfile together. Merging the release PR creates the tag and GitHub release. The workflow then builds the tagged source, verifies package checksums, and attaches provenance attestations and downloads to the release.

| Platform | Packages |
| --- | --- |
| Linux x86-64 | `.tar.gz` and `.deb`, built on Ubuntu 24.04 |
| macOS | `.tar.gz`, zipped `.app`, and DMG |
| Windows x86-64 | `.zip` and MSI |

The macOS app bundle and DMG use the same organization-level Apple credentials as Zmail. The bundle uses Developer ID signing and notarization when both sets of credentials are configured, and ad-hoc signing when the certificate is absent. Windows and Debian installers currently have no platform signature. GitHub publishes provenance attestations for all release assets, which you can verify with `gh attestation verify <file> --repo unicornops/zim`.

To verify packaging with no public release, use the Release Please workflow's dry run. A dry run of `main` also exercises the macOS signing path when credentials are available. To rebuild assets for an existing release, provide its `vX.Y.Z` tag and set `dry_run=false`.

```sh
gh workflow run release-please.yml --ref main -f tag_name=main -f dry_run=true
```

Release Please uses the UnicornOps release app when its organization secrets are available. Its token is scoped to this repository so updates to release PRs trigger CI. Outside UnicornOps, a configured `RELEASE_PLEASE_TOKEN` takes precedence over the repository workflow token.
