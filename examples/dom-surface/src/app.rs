use nestix::{Element, callback, component, computed, create_state, layout, unmount_root};
use nestix_native::dom::{DomAttribute, DomElement, DomEvent, DomProperty};
use nestix_native::{
    AlignItems, Button, DomSurface, DomTemplate, FlexDirection, FlexView, Input, Root,
    StyleProvider, Text, WebViewController, Window, style,
};

#[component]
pub fn App() -> Element {
    let count = create_state(0);
    let name = create_state("Nestix".to_string());
    let dom_surface = WebViewController::new();
    let count_for_custom_element = count.clone();
    let styles = style! {
        .app {
            padding: 20 px;
            gap: 16 px;
        }

        .native_panel, .dom_panel {
            padding: 16 px;
            gap: 10 px;
        }

        .native_actions, .dom_actions {
            flex_direction: row;
            align_items: center;
            gap: 8 px;
        }

        .heading {
            font_size: 20 px;
            font_weight: semi-bold;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix DomSurface",
                    .desktop(
                        .width = 620,
                        .height = 640,
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    ),
                ) {
                    FlexView(
                        .class = "app",
                        .view(.flex_grow = 1.0),
                        .align_items = AlignItems::Stretch,
                    ) {
                        FlexView(.class = "native_panel") {
                            Text("Native controls", .class = "heading")
                            Text(computed!([count] || format!("Shared count: {}", count.get())))
                            Input(
                                .value = name.clone(),
                                .on_text_change = callback!(
                                    [name] |value: &str| {
                                        name.set(value.to_string());
                                    }
                                ),
                            )
                            FlexView(
                                .class = "native_actions",
                                .flex_direction = FlexDirection::Row,
                            ) {
                                Button(
                                    .title = "Increment natively",
                                    .on_click = callback!(
                                        [count] || {
                                            count.update(|value| value + 1)
                                        }
                                    ),
                                )
                                Button(
                                    .title = "Reset",
                                    .on_click = callback!([count] || count.set(0)),
                                )
                                Button(
                                    .title = "Open DOM DevTools",
                                    .on_click = callback!(
                                        [dom_surface] || {
                                            if let Err(error) = dom_surface.open_dev_tools() {
                                                eprintln!("Could not open DOM DevTools: {error}");
                                            }
                                        }
                                    ),
                                )
                            }
                        }
                        DomSurface(
                            .class = "dom_surface",
                            .view(.height = 330, .align_self = AlignItems::Stretch),
                            .template = DomTemplate::resource("web/index.html").with_development_path(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/web/index.html")),
                            .inspectable = true,
                            .controller = dom_surface,
                        ) {
                            FlexView(.class = "dom_panel") {
                                Text("DOM elements in DomSurface", .class = "heading")
                                Text(computed!([name] || format!("Hello, {}", name.get())))
                                Text(
                                    computed!(
                                        [count] || format!("Shared count: {}", count.get())
                                    ),
                                )
                                Input(
                                    .value = name.clone(),
                                    .on_text_change = callback!(
                                        [name] |value: &str| {
                                            name.set(value.to_string());
                                        }
                                    ),
                                )
                                DomElement(
                                    "nestix-counter-action",
                                    .dom_class = "counter-action",
                                    .attributes = computed!(
                                        [count]
                                            || vec![DomAttribute::string(
                                                "aria-label",
                                                format!(
                                                    "Increment custom element from {}",
                                                    count.get()
                                                ),
                                            ),]
                                    ),
                                    .properties = computed!(
                                        [count]
                                            || vec![DomProperty::new("currentCount", count.get()),]
                                    ),
                                    .events = vec![DomEvent::new("increment", move |_| {
                                        count_for_custom_element.update(|value| value + 1)
                                    })],
                                ) {
                                    Text(
                                        computed!(
                                            [count]
                                                || format!(
                                                    "Custom element · count {} · select to increment",
                                                    count.get()
                                                )
                                        ),
                                    )
                                }
                                FlexView(
                                    .class = "dom_actions",
                                    .flex_direction = FlexDirection::Row,
                                ) {
                                    Button(
                                        .title = "Add ten in the DOM",
                                        .on_click = callback!(
                                            [count] || { count.update(|value| value + 10) }
                                        ),
                                    )
                                    Button(
                                        .title = "Reset",
                                        .on_click = callback!([count] || count.set(0)),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
