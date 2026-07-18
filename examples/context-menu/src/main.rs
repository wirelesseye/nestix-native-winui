use env_logger::Env;
use nestix::{
    ContextProvider, Element, callback, component, computed, create_state, layout, mount_root,
};
use nestix_native::{
    AlignItems, BackendContext, Button, CheckMenuItem, Color, ContextMenu, ContextMenuController,
    ContextMenuPosition, FlexView, JustifyContent, Menu, MenuItem, MenuSeparator, RGBColor,
    RadioMenuItem, Root, Shortcut, Submenu, Text, Window, default_backend,
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
            ContextMenuExample
        }
    });
}

#[component]
fn ContextMenuExample(_: &(), element: &Element) -> Element {
    let status = create_state("No command selected".to_string());
    let show_details = create_state(true);
    let show_advanced = create_state(false);
    let sort_order = create_state("Name".to_string());
    let context_menu = ContextMenuController::new();

    let menu = layout! {
        Menu {
            MenuItem(
                "Open",
                .shortcut = Shortcut::primary('O'),
                .on_activate = callback!([status] || {
                    status.set("Open selected".to_string());
                }),
            )
            MenuItem(
                "Rename",
                .on_activate = callback!([status] || {
                    status.set("Rename selected".to_string());
                }),
            )
            MenuItem("Unavailable command", .enabled = false)
            MenuSeparator()
            CheckMenuItem(
                "Show details",
                .checked = show_details.clone(),
                .on_checked_change = callback!([show_details, status] |checked| {
                    show_details.set(checked);
                    status.set(format!(
                        "Details {}",
                        if checked { "shown" } else { "hidden" }
                    ));
                }),
            )
            CheckMenuItem(
                "Show advanced command",
                .checked = show_advanced.clone(),
                .on_checked_change = callback!([show_advanced] |checked| {
                    show_advanced.set(checked);
                }),
            )
            MenuItem(
                "Advanced command",
                .visible = show_advanced.clone(),
                .on_activate = callback!([status] || {
                    status.set("Advanced command selected".to_string());
                }),
            )
            MenuSeparator()
            Submenu("Sort by") {
                RadioMenuItem(
                    "Name",
                    .group = "sort-order",
                    .selected = computed!([sort_order] || sort_order.get() == "Name"),
                    .on_select = callback!([sort_order, status] || {
                        sort_order.set("Name".to_string());
                        status.set("Sorting by name".to_string());
                    }),
                )
                RadioMenuItem(
                    "Date",
                    .group = "sort-order",
                    .selected = computed!([sort_order] || sort_order.get() == "Date"),
                    .on_select = callback!([sort_order, status] || {
                        sort_order.set("Date".to_string());
                        status.set("Sorting by date".to_string());
                    }),
                )
            }
        }
    };

    layout! {
        Root {
            Window(
                .title = "Nestix Context Menu",
                .width = 520,
                .height = 360,
                .on_close_requested = callback!([element] || element.unmount()),
            ) {
                FlexView(
                    .align_items = AlignItems::Center,
                    .justify_content = JustifyContent::Center,
                    .bg_color = Some(Color::RGB(RGBColor::from_rgb(238, 242, 247))),
                    .view(.flex_grow = 1.0),
                ) {
                    ContextMenu(
                        .menu = menu,
                        .controller = context_menu.clone(),
                    ) {
                        FlexView(
                            .align_items = AlignItems::Center,
                            .justify_content = JustifyContent::Center,
                            .bg_color = Some(Color::WHITE),
                            .view(.width = 360, .height = 220),
                            .container(.padding = 24),
                        ) {
                            Text("Right-click this card or use the button")
                            Button(
                                .title = "Show context menu",
                                .on_click = callback!([context_menu, status] || {
                                    if let Err(error) = context_menu.show(ContextMenuPosition::Cursor) {
                                        status.set(format!("Could not show menu: {error}"));
                                    }
                                }),
                            )
                            Text("Try commands, checkboxes, radio items, and the submenu.")
                            Text(computed!([status] || format!("Status: {}", status.get())))
                            Text(computed!([show_details, sort_order] || {
                                if show_details.get() {
                                    format!("Details: sorted by {}", sort_order.get())
                                } else {
                                    "Details are hidden".to_string()
                                }
                            }))
                        }
                    }
                }
            }
        }
    }
}
