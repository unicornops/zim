# Zim

Zim is a cross-platform instant messaging project for macOS, Windows, and Linux, built in Rust with Zed's [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui). Its goal is to bring public chat services into one desktop app through pluggable protocol adapters. The current desktop shell opens a People pane and an empty conversation area.

Open a conversation with a person, even if you know them on several networks. Each person can have several linked chat identities. Zim can choose an available route for each outgoing message while keeping replies in one timeline. The composer shows the account and network it will use, and you can choose or pin a route yourself. This contact model follows [Everybuddy and Ayttm](https://ayttm.sourceforge.net/ayttm-manual/ii.php).

The first messaging release targets one-to-one text across XMPP and Matrix, with a local demo adapter for testing. See the [project plan](docs/plan.md), [protocol roadmap](docs/protocols.md), and [first milestone](https://github.com/unicornops/zim/milestone/1).

## Development

Install [Rust through rustup](https://rustup.rs/) and the [platform prerequisites](docs/development.md). Launch the desktop shell with `cargo run --locked`. GitHub Actions tests and builds the workspace on all three platforms. See the [development and release guide](docs/development.md).

## License

Zim is licensed under [GPL-3.0-only](LICENSE), the same license as Zmail. GPUI is distributed under [Apache-2.0](licenses/GPUI-Apache-2.0.txt).
