# Zim

Zim is a planned cross-platform instant messaging client for macOS, Windows, and Linux, built in Rust with Zed's [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui). Its goal is to bring public chat services into one desktop app through pluggable protocol adapters.

Open a conversation with a person, even if you know them on several networks. Each person can have several linked chat identities. Zim can choose an available route for each outgoing message while keeping replies in one timeline. The composer shows the account and network it will use, and you can choose or pin a route yourself. This contact model follows [Everybuddy and Ayttm](https://ayttm.sourceforge.net/ayttm-manual/ii.php).

The first release target is one-to-one text messaging across XMPP and Matrix, with a local demo adapter for testing. Zim is at the planning stage. See the [project plan](docs/plan.md), [protocol roadmap](docs/protocols.md), and [first milestone](https://github.com/unicornops/zim/milestones).
