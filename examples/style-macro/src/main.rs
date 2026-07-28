use env_logger::Env;
use nestix::{Element, callback, component, layout, mount_root, unmount_root};
use nestix_native::{Button, FlexView, Root, StyleProvider, Text, Window, style};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND,  },
        ) {
            StyleMacroApp
        }
    });
}

#[component]
fn StyleMacroApp() -> Element {
    let styles = style! {
        // Class selectors and selector lists.
        .app {
            // Rust tokenizes `em` as an exponent marker when attached to a number,
            // so the style DSL accepts the CSS unit with whitespace.
            padding: 2 em;
            gap: 1.25 em;
        }

        .heading, .card_title {
            font_weight: semi-bold;
            text_color: #172033;
        }

        .heading {
            font_size: 2 em;
        }

        .intro {
            text_color: #526079;
        }

        // Nested child, compound, pseudo-class, and sibling selectors.
        .gallery {
            gap: 0.75 em;

            > .card {
                padding: 1 em;
                gap: 0.45 em;
                bg_color: #EEF2F8;

                &:first_child {
                    bg_color: #E5F0FF;
                }

                &.featured {
                    bg_color: #E8F7EE;
                }

                + .card {
                    margin_top: 0.25 em;
                }

                > .card_title {
                    font_size: 1.25 em;
                }

                // An implicit nested selector is a descendant selector.
                .detail {
                    text_color: #526079;
                }

                // `>>` spells an explicit descendant combinator.
                >> .action {
                    margin_top: 0.35 em;
                }
            }
        }

        // Negation and a direct-child selector outside of nesting.
        .card:not(.featured) > .tag {
            text_color: #315FA8;
        }

        .card.featured > .tag {
            text_color: #18733A;
            font_weight: bold;
        }

        .actions {
            flex_direction: row;
            align_items: center;
            gap: 0.5 em;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix style! selector gallery",
                    .desktop(
                        .width = 620,
                        .height = 650,
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    )
                ) {
                    FlexView(.class = "app", .view(.flex_grow = 1.0)) {
                        Text("style! selector gallery", .class = "heading")
                        Text(
                            "Class, compound, pseudo-class, combinator, selector-list, and nested rules.",
                            .class = "intro",
                        )
                        FlexView(.class = "gallery") {
                            FlexView(.class = "card") {
                                Text("First child", .class = "card_title")
                                Text(
                                    ":first_child changes this card's background.",
                                    .class = "detail",
                                )
                                Text(":not(.featured) > .tag", .class = "tag")
                            }
                            FlexView(.class = "card featured") {
                                Text("Compound selector", .class = "card_title")
                                Text(
                                    "&.featured combines the nested parent with a class.",
                                    .class = "detail",
                                )
                                Text(".card.featured > .tag", .class = "tag")
                                FlexView(.class = "actions") {
                                    Button(.title = "Nested action", .class = "action")
                                    Button(.title = "Sibling action", .class = "action")
                                }
                            }
                            FlexView(.class = "card") {
                                Text("Combinators", .class = "card_title")
                                FlexView {
                                    Text(
                                        "> targets direct children; >> targets descendants.",
                                        .class = "detail",
                                    )
                                    Button(.title = "Descendant button", .class = "action")
                                }
                                Text("+ adds spacing between adjacent cards.", .class = "tag")
                            }
                        }
                    }
                }
            }
        }
    }
}
