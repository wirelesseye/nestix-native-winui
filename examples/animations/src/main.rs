use std::time::Duration;

use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, AnimationSpec, Button, Easing, FlexDirection, FlexView, Length, Root,
    StyleProvider, Text, Window, animate, style,
};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND },
        ) {
            AnimationApp
        }
    });
}

#[component]
fn AnimationApp() -> Element {
    let expanded = create_state(false);
    let large_window = create_state(false);
    let window_width = create_state(680.0);
    let window_height = create_state(460.0);
    let card_class = computed!(
        [expanded] || {
            if expanded.get() {
                "card expanded"
            } else {
                "card"
            }
        }
    );

    let styles = style! {
        .demo_window {
            transition: width 420ms ease_in_out, height 420ms ease_in_out;
        }

        .root {
            padding: 28 px;
            gap: 22 px;
            bg_color: #F2F4F8;
        }

        .stage {
            height: 210 px;
            padding: 18 px;
            bg_color: #FFFFFF;
        }

        .card {
            width: 150 px;
            height: 92 px;
            margin_left: 0 px;
            padding: 16 px;
            bg_color: #3767D6;
            transition: layout 500ms ease_in_out;
        }

        .card.expanded {
            width: 330 px;
            height: 150 px;
            margin_left: 210 px;
            padding: 28 px;
        }

        .card_text {
            text_color: white;
        }

        .controls {
            gap: 12 px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .class = "demo_window",
                    .title = "Nestix Native Animations",
                    .width = window_width.clone(),
                    .height = window_height.clone(),
                    .on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    }),
                    .resizable = false,
                ) {
                    FlexView(.class = "root", .view(.flex_grow = 1.0)) {
                        Text(
                            "Cross-platform animation API",
                            .font(.font_size = Some(Length::logical(24.0))),
                        )
                        Text("The card uses stylesheet transitions; the window uses animate(...).")
                        FlexView(.class = "stage") {
                            FlexView(.class = card_class) {
                                Text("Animated layout", .class = "card_text")
                            }
                        }
                        FlexView(
                            .class = "controls",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Button(
                                .title = "Toggle card",
                                .on_click = callback!(
                                    [expanded] || {
                                        expanded.set(!expanded.get());
                                    }
                                ),
                            )
                            Button(
                                .title = "Animate window",
                                .on_click = callback!(
                                    [large_window, window_width, window_height] || {
                                        let next = !large_window.get();
                                        large_window.set(next);
                                        animate(
                                            AnimationSpec::new(Duration::from_millis(420))
                                                .easing(Easing::EaseInOut),
                                            || {
                                                window_width.set(if next { 900.0 } else { 680.0 });
                                                window_height.set(if next { 620.0 } else { 460.0 });
                                            },
                                        );
                                    }
                                ),
                            )
                        }
                    }
                }
            }
        }
    }
}
