//! The desktop shell for Zim.

use gpui::{
    App, AppContext as _, Bounds, Context, IntoElement, KeyBinding, Menu, MenuItem,
    ParentElement as _, Render, Styled as _, TitlebarOptions, Window, WindowBounds, WindowHandle,
    WindowOptions, actions, div, px, rgb, size,
};

const APP_NAME: &str = "Zim";
const APP_ID: &str = "ie.unicornops.zim";

actions!(
    zim,
    [
        #[doc = "Quit the Zim desktop application."]
        Quit
    ]
);

/// Launch Zim with the native GPUI platform and accessibility backend.
pub fn run() {
    gpui_platform::application().run(|cx| {
        cx.set_app_identity(APP_ID, APP_NAME);
        cx.on_action(|_: &Quit, cx| cx.quit());
        let shortcut = if cfg!(target_os = "macos") {
            "cmd-q"
        } else {
            "ctrl-q"
        };
        cx.bind_keys([KeyBinding::new(shortcut, Quit, None)]);
        cx.set_menus(vec![Menu {
            name: APP_NAME.into(),
            items: vec![MenuItem::action("Quit Zim", Quit)],
            disabled: false,
        }]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        if let Err(error) = open_workspace(cx) {
            eprintln!("Unable to open Zim: {error:#}");
            std::process::exit(1);
        }
        cx.activate(true);
    });
}

fn open_workspace(cx: &mut App) -> anyhow::Result<WindowHandle<Workspace>> {
    let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
    cx.open_window(
        WindowOptions {
            app_id: Some(APP_ID.into()),
            titlebar: Some(TitlebarOptions {
                title: Some(APP_NAME.into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(600.), px(400.))),
            ..Default::default()
        },
        |_, cx| cx.new(|_| Workspace),
    )
}

struct Workspace;

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(rgb(0x20242b))
            .text_color(rgb(0xf0f2f5))
            .child(
                div()
                    .w(px(230.))
                    .h_full()
                    .border_r_1()
                    .border_color(rgb(0x343b45))
                    .bg(rgb(0x191d23))
                    .p_5()
                    .text_lg()
                    .child("People"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0xaeb8c5))
                    .child("No conversations yet"),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::open_workspace;

    #[gpui::test]
    fn opens_the_desktop_workspace(cx: &mut TestAppContext) {
        let window = cx.update(|cx| open_workspace(cx).expect("Zim window opens"));

        window
            .update(cx, |_, window, _| {
                assert!(window.viewport_size().width > gpui::px(0.));
            })
            .expect("Zim workspace is mounted");
    }
}
