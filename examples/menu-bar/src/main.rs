use env_logger::Env;
use nestix::{
    ContextProvider, Element, callback, component, computed, create_state, effect, layout,
    mount_root,
};
use nestix_native::{
    AlignItems, BackendContext, CheckMenuItem, Color, FlexView, JustifyContent, Menu, MenuBar,
    MenuItem, MenuSeparator, RGBColor, Root, Shortcut, Submenu, Text, Window, default_backend,
};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let backend = if cfg!(target_os = "windows") {
        &WINUI_BACKEND
    } else {
        default_backend()
    };
    mount_root(&layout! {
        ContextProvider<BackendContext>(BackendContext { backend }) {
            MenuBarExample
        }
    });
}

#[component]
fn MenuBarExample(_: &(), element: &Element) -> Element {
    let status = create_state("Choose a menu command".to_string());
    let show_status = create_state(true);
    let window_menu_open = create_state(true);
    let plain_window_open = create_state(true);

    effect!(
        [element, window_menu_open, plain_window_open] || {
            if !window_menu_open.get() && !plain_window_open.get() {
                element.unmount();
            }
        }
    );

    let application_menu = layout! {
        Submenu("Application") {
            MenuItem(
                "About Menu Bar Example",
                .on_activate = callback!([status] || {
                    status.set("Application-wide About selected".to_string());
                }),
            )
            MenuSeparator()
            MenuItem(
                "Application Action",
                .shortcut = Shortcut::primary('A'),
                .on_activate = callback!([status] || {
                    status.set("Application-wide action selected".to_string());
                }),
            )
        }
    };

    layout! {
        Root {
            // macOS only: This menu bar is available application-wide.
            // A window without its own menu bar uses it when focused.
            MenuBar(.menu = layout! {
                Menu {
                    $(application_menu.clone())
                }
            })

            if window_menu_open.get() {
                Window(
                    .title = "Window-specific menu",
                    .width = 480,
                    .height = 300,
                    .on_close_requested = callback!([window_menu_open] || {
                        window_menu_open.set(false);
                    }),
                ) {
                    FlexView {
                    // On macOS: it is active only while this window is focused.
                    MenuBar(.menu = layout! {
                        Menu {
                            $(application_menu)
                            Submenu("File") {
                                MenuItem(
                                    "New Document",
                                    .shortcut = Shortcut::primary('N'),
                                    .on_activate = callback!([status] || {
                                        status.set("New Document selected".to_string());
                                    }),
                                )
                                MenuItem(
                                    "Save",
                                    .shortcut = Shortcut::primary('S'),
                                    .on_activate = callback!([status] || {
                                        status.set("Save selected".to_string());
                                    }),
                                )
                                MenuSeparator()
                                CheckMenuItem(
                                    "Show status",
                                    .checked = show_status.clone(),
                                    .on_checked_change = callback!([show_status] |checked| {
                                        show_status.set(checked);
                                    }),
                                )
                            }
                        }
                    })
                    FlexView(
                        .align_items = AlignItems::Center,
                        .justify_content = JustifyContent::Center,
                        .bg_color = Some(Color::RGB(RGBColor::from_rgb(238, 242, 247))),
                        .view(.flex_grow = 1.0),
                    ) {
                        Text("This window supplies its own File menu.")
                        if show_status.get() {
                            Text(computed!([status] || format!("Status: {}", status.get())))
                        }
                    }
                    }
                }
            }

            if plain_window_open.get() {
                Window(
                    .title = "No window menu",
                    .width = 480,
                    .height = 240,
                    .on_close_requested = callback!([plain_window_open] || {
                        plain_window_open.set(false);
                    }),
                ) {
                    FlexView(
                        .align_items = AlignItems::Center,
                        .justify_content = JustifyContent::Center,
                        .bg_color = Some(Color::RGB(RGBColor::from_rgb(247, 244, 238))),
                        .view(.flex_grow = 1.0),
                    ) {
                        Text("This window has no window-specific menu bar.")
                        Text(computed!([status] || format!("Status: {}", status.get())))
                    }
                }
            }
        }
    }
}
