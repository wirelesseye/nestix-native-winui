use env_logger::Env;
use nestix::{
    ContextProvider, Element, callback, component, computed, create_state, layout, mount_root,
};
use nestix_native::{
    AlignItems, BackendContext, Button, Checkbox, FlexDirection, FlexView, Input, RadioButton,
    Root, Select, SelectOption, Slider, StyleProvider, Switch, Text, Window, default_backend,
    style,
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
            FormControlsApp
        }
    });
}

#[component]
fn FormControlsApp() -> Element {
    let name = create_state(String::new());
    let newsletter = create_state(false);
    let notifications = create_state(true);
    let density = create_state("comfortable".to_string());
    let country = create_state(None::<String>);
    let volume = create_state(50.0);
    let status = create_state("Complete the form, then press Save.".to_string());

    let styles = style! {
        .content {
            padding: 28px;
        }

        .heading {
            font_size: 24px;
            margin_bottom: 6px;
        }

        .description {
            margin_bottom: 22px;
        }

        .label {
            margin_bottom: 6px;
        }

        .field {
            margin_bottom: 16px;
        }

        .choice {
            margin_right: 18px;
        }

        .actions {
            margin_top: 8px;
            margin_bottom: 18px;
        }

        .actions > .__Button {
            margin_right: 10px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix Form Controls",
                    .width = 560,
                    .height = 680,
                ) {
                    FlexView(.class = "content", .view(.flex_grow = 1.0)) {
                        Text("Form controls", .class = "heading")
                        Text(
                            "Controlled native components exposed through nestix-native.",
                            .class = "description",
                        )

                        Text("Name", .class = "label")
                        Input(
                            .class = "field",
                            .view(.width = 320),
                            .value = name.clone(),
                            .on_text_change = callback!([name] |value: &str| {
                                name.set(value.to_string());
                            }),
                        )

                        Checkbox(
                            "Subscribe to the newsletter",
                            .class = "field",
                            .checked = newsletter.clone(),
                            .on_checked_change = callback!([newsletter] |checked| {
                                newsletter.set(checked);
                            }),
                        )

                        Text("Interface density", .class = "label")
                        FlexView(
                            .class = "field",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            RadioButton(
                                "Compact",
                                .class = "choice",
                                .group = "density",
                                .selected = computed!([density] || density.get() == "compact"),
                                .on_select = callback!([density] || {
                                    density.set("compact".to_string());
                                }),
                            )
                            RadioButton(
                                "Comfortable",
                                .group = "density",
                                .selected = computed!([density] || density.get() == "comfortable"),
                                .on_select = callback!([density] || {
                                    density.set("comfortable".to_string());
                                }),
                            )
                        }

                        Text("Country", .class = "label")
                        Select(
                            .class = "field",
                            .view(.width = 220),
                            .value = country.clone(),
                            .on_value_change = callback!([country] |value: &str| {
                                country.set(Some(value.to_string()));
                            }),
                        ) {
                            SelectOption("Australia", .value = "au")
                            SelectOption("New Zealand", .value = "nz")
                            SelectOption("United States", .value = "us")
                            SelectOption("Unavailable choice", .value = "disabled", .enabled = false)
                        }

                        Text(
                            computed!([volume] || format!("Volume: {:.0}", volume.get())),
                            .class = "label",
                        )
                        Slider(
                            .class = "field",
                            .view(.width = 320),
                            .value = volume.clone(),
                            .minimum = 0.0,
                            .maximum = 100.0,
                            .on_value_change = callback!([volume] |value| {
                                volume.set(value);
                            }),
                        )

                        FlexView(
                            .class = "field",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Text("Enable notifications", .class = "choice")
                            Switch(
                                .checked = notifications.clone(),
                                .on_checked_change = callback!([notifications] |checked| {
                                    notifications.set(checked);
                                }),
                            )
                        }

                        FlexView(
                            .class = "actions",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Button(
                                .title = "Save",
                                .disabled = computed!([name] || name.get().trim().is_empty()),
                                .on_click = callback!([
                                    name,
                                    newsletter,
                                    notifications,
                                    density,
                                    country,
                                    volume,
                                    status
                                ] || {
                                    let country = country
                                        .get()
                                        .unwrap_or_else(|| "not selected".to_string());
                                    status.set(format!(
                                        "Saved: name={:?}, newsletter={}, notifications={}, density={}, country={}, volume={:.0}",
                                        name.get(),
                                        newsletter.get(),
                                        notifications.get(),
                                        density.get(),
                                        country,
                                        volume.get(),
                                    ));
                                }),
                            )
                            Button(
                                .title = "Reset",
                                .disabled = computed!([
                                    name,
                                    newsletter,
                                    notifications,
                                    density,
                                    country,
                                    volume
                                ] || {
                                    name.get().is_empty()
                                        && !newsletter.get()
                                        && notifications.get()
                                        && density.get() == "comfortable"
                                        && country.get().is_none()
                                        && volume.get() == 50.0
                                }),
                                .on_click = callback!([
                                    name,
                                    newsletter,
                                    notifications,
                                    density,
                                    country,
                                    volume,
                                    status
                                ] || {
                                    name.set(String::new());
                                    newsletter.set(false);
                                    notifications.set(true);
                                    density.set("comfortable".to_string());
                                    country.set(None);
                                    volume.set(50.0);
                                    status.set("Form reset.".to_string());
                                }),
                            )
                        }

                        Text(status)
                    }
                }
            }
        }
    }
}
