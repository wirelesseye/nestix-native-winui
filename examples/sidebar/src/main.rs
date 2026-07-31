use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    FlexView, Input, NavigationItem, Root, Sidebar, SidebarNavigation, Text, TitlebarMode, Window,
};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND,  },
        ) {
            ExampleApp
        }
    });
}

#[component]
fn ExampleApp() -> Element {
    let (page, set_page) = create_state(Some("home".to_string()));

    layout! {
        Root {
            Window(
                .title = "Nestix Sidebar",
                .desktop(
                    .width = 800,
                    .height = 520,
                    .on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    }),
                    .titlebar_mode = TitlebarMode::Overlay,
                ),
            ) {
                FlexView(.view(.flex_grow = 1.0)) {
                    Sidebar(.width = 260.0, .min_width = 260.0, .resizable = true) {
                        FlexView(.view(.flex_grow = 1.0), .gap = 15) {
                            Input(.view(.margin_horizontal = 15))
                            SidebarNavigation(
                                .view(.flex_grow = 1.0),
                                .value = page.clone(),
                                .on_value_change = callback!(
                                    [set_page] |value: &str| {
                                        set_page.set(Some(value.to_string()));
                                    }
                                ),
                            ) {
                                NavigationItem("Home", .value = "home")
                                NavigationItem("Projects", .value = "projects")
                                NavigationItem("Settings", .value = "settings")
                            }
                        }
                    }
                    FlexView(.view(.flex_grow = 1.0)) {
                        Text(
                            computed!(
                                [page]
                                    || format!(
                                        "Selected page: {}",
                                        page.get().as_deref().unwrap_or("none")
                                    )
                            ),
                        )
                    }
                }
            }
        }
    }
}
