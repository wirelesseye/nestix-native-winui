use std::rc::Rc;

use env_logger::Env;
use nestix::{
    ContextProvider, Element, callback, component, computed, create_state, layout, mount_root,
};
use nestix_native::{
    AlignItems, BackendContext, Button, Color, FlexDirection, FlexView, RGBColor, Root,
    StyleProvider, Text, Window, default_backend, style,
};
use nestix_native_winui::WinUiBackend;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let backend = if cfg!(target_os = "windows") {
        Rc::new(WinUiBackend)
    } else {
        default_backend()
    };
    mount_root(&layout! {
        ContextProvider<BackendContext>(BackendContext::new(backend)) {
            ExampleApp
        }
    });
}

#[component]
fn ExampleApp() -> Element {
    let count = create_state(0);
    let message = create_state("Ready".to_string());
    let styles = style! {
        .surface {
            // bg_color: #F4F6F8;
        }

        .panel {
            margin: 24px;
        }

        .stack > .__Text, .stack > .__Button {
            margin_bottom: 12px;
        }

        .actions > .__Button {
            margin_right: 8px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix Counter",
                    .width = 420,
                    .height = 320,
                    .on_resize = callback!(|size| {
                        println!("{:?}", size);
                    }),
                ) {
                    FlexView(.class = "surface", .view(.grow = 1.0)) {
                        FlexView(.class = "panel stack", .align_items = AlignItems::Start) {
                            Text("Nestix native")
                            Text(computed!([count] || format!("Count: {}", count.get())))
                            Text(message.clone())
                            FlexView(
                                .class = "actions",
                                .flex_direction = FlexDirection::Row,
                                .align_items = AlignItems::Center,
                            ) {
                                Button(
                                    .title = "Increment",
                                    .on_click = callback!([count, message] || {
                                        count.mutate(|count| *count += 1);
                                        message.set("Counter updated".to_string());
                                    })
                                )
                                Button(
                                    .title = "Reset",
                                    .on_click = callback!([count, message] || {
                                        count.set(0);
                                        message.set("Ready".to_string());
                                    })
                                )
                            }
                            FlexView(
                                .view(.width = 120, .height = 6),
                                .bg_color = Some(Color::RGB(RGBColor::from_rgb(0, 120, 212))),
                            )
                        }
                    }
                }
            }
        }
    }
}
