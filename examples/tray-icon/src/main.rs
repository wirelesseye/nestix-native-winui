use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, Button, CheckMenuItem, FlexView, ImageSource, JustifyContent, Menu, MenuItem,
    MenuSeparator, RadioMenuItem, Root, Submenu, Text, TrayIcon, Window,
};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND,  },
        ) {
            TrayIconExample
        }
    });
}

#[component]
fn TrayIconExample() -> Element {
    let (activation_count, set_activation_count) = create_state(0);
    let (icon_visible, set_icon_visible) = create_state(true);
    let (menu_on_primary, set_menu_on_primary) = create_state(false);
    let (mode, set_mode) = create_state("Available".to_string());
    let (show_window, set_show_window) = create_state(true);

    let tray_menu = layout! {
        Menu {
            MenuItem(
                "Increment activation count",
                .on_activate = callback!(
                    [set_activation_count] || {
                        set_activation_count.mutate(|count| *count += 1);
                    }
                ),
            )
            CheckMenuItem(
                "Open menu on primary activation",
                .checked = menu_on_primary.clone(),
                .on_checked_change = callback!(
                    [set_menu_on_primary] | checked | {
                        set_menu_on_primary.set(checked);
                    }
                ),
            )
            Submenu("Mode") {
                RadioMenuItem(
                    "Available",
                    .group = "tray-mode",
                    .selected = computed!([mode] || mode.get() == "Available"),
                    .on_select = callback!(
                        [set_mode] || set_mode.set("Available".to_string())
                    ),
                )
                RadioMenuItem(
                    "Busy",
                    .group = "tray-mode",
                    .selected = computed!([mode] || mode.get() == "Busy"),
                    .on_select = callback!([set_mode] || set_mode.set("Busy".to_string())),
                )
            }
            MenuSeparator()
            MenuItem(
                "Quit",
                .on_activate = callback!(|| {
                    unmount_root().expect("root should be mounted");
                }),
            )
        }
    };

    layout! {
        Root {
            TrayIcon(
                .icon = ImageSource::bytes(include_bytes!("../assets/tray-icon.png").as_slice()),
                .tooltip = "Nestix tray example".to_string(),
                .visible = icon_visible.clone(),
                .menu = tray_menu,
                .on_activate = callback!(
                    [
                        menu_on_primary,
                        set_show_window,
                        set_activation_count,
                    ] |event: nestix_native::TrayIconEvent| {
                        set_show_window.set(true);
                        if menu_on_primary.get() {
                            if let Err(error) = event.show_menu() {
                                eprintln!("could not show tray menu: {error}");
                            }
                        } else {
                            set_activation_count.mutate(|count| *count += 1);
                        }
                    }
                ),
                .on_secondary = callback!(|event: nestix_native::TrayIconEvent| {
                    if let Err(error) = event.show_menu() {
                        eprintln!("could not show tray menu: {error}");
                    }
                }),
            )
            if show_window.get() {
                Window(
                    .title = "Nestix Tray Icon",
                    .desktop(
                        .width = 460,
                        .height = 300,
                        .on_close_requested = callback!(
                            [set_show_window] || set_show_window.set(false)
                        ),
                    ),
                ) {
                    FlexView(
                        .align_items = AlignItems::Center,
                        .justify_content = JustifyContent::Center,
                        .view(.flex_grow = 1.0),
                    ) {
                        Text("Primary activation increments unless the menu option is enabled.")
                        Text("Secondary activation opens the same reactive Menu.")
                        Text(
                            computed!(
                                [activation_count] || {
                                    format!("Primary activations: {}", activation_count.get())
                                }
                            ),
                        )
                        Text(computed!([mode] || format!("Mode: {}", mode.get())))
                        Button(
                            .title = computed!(
                                [icon_visible] || {
                                    if icon_visible.get() {
                                        "Hide tray icon"
                                    } else {
                                        "Show tray icon"
                                    }
                                }
                            ),
                            .on_click = callback!(
                                [set_icon_visible] || {
                                    set_icon_visible.mutate(|visible| *visible = !*visible);
                                }
                            ),
                        )
                    }
                }
            }
        }
    }
}
