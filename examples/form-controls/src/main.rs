use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, BackendCase, Button, Checkbox, FlexDirection, FlexView, Input, RadioButton, Root,
    Select, SelectOption, Slider, StyleProvider, Switch, Text, Window, style,
};
use nestix_native_winui::WINUI_BACKEND;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND,  },
        ) {
            FormControlsApp
        }
    });
}

#[component]
fn FormControlsApp() -> Element {
    let (name, set_name) = create_state(String::new());
    let (newsletter, set_newsletter) = create_state(false);
    let (notifications, set_notifications) = create_state(true);
    let (density, set_density) = create_state("comfortable".to_string());
    let (country, set_country) = create_state(None::<String>);
    let (volume, set_volume) = create_state(50.0);
    let (status, set_status) = create_state("Complete the form, then press Save.".to_string());

    let styles = style! {
        .content {
            padding: 28 px;
        }

        .heading {
            font_size: 24 px;
            margin_bottom: 6 px;
        }

        .description {
            margin_bottom: 22 px;
        }

        .label {
            margin_bottom: 6 px;
        }

        .field {
            margin_bottom: 16 px;
        }

        .choice {
            margin_right: 18 px;
        }

        .actions {
            margin_top: 8 px;
            margin_bottom: 18 px;
        }

        .actions > .__Button {
            margin_right: 10 px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix Form Controls",
                    .desktop(
                        .width = 560,
                        .height = 680,
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    ),
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
                            .on_text_change = callback!(
                                [set_name] |value: &str| {
                                    set_name.set(value.to_string());
                                }
                            ),
                        )
                        Checkbox(
                            "Subscribe to the newsletter",
                            .class = "field",
                            .checked = newsletter.clone(),
                            .on_checked_change = callback!(
                                [set_newsletter] | checked | {
                                    set_newsletter.set(checked);
                                }
                            ),
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
                                .selected = computed!(
                                    [density] || density.get() == "compact"
                                ),
                                .on_select = callback!(
                                    [set_density] || {
                                        set_density.set("compact".to_string());
                                    }
                                ),
                            )
                            RadioButton(
                                "Comfortable",
                                .group = "density",
                                .selected = computed!(
                                    [density] || density.get() == "comfortable"
                                ),
                                .on_select = callback!(
                                    [set_density] || {
                                        set_density.set("comfortable".to_string());
                                    }
                                ),
                            )
                        }
                        Text("Country", .class = "label")
                        Select(
                            .class = "field",
                            .view(.width = 220),
                            .value = country.clone(),
                            .on_value_change = callback!(
                                [set_country] |value: &str| {
                                    set_country.set(Some(value.to_string()));
                                }
                            ),
                        ) {
                            SelectOption("Australia", .value = "au")
                            SelectOption("New Zealand", .value = "nz")
                            SelectOption("United States", .value = "us")
                            SelectOption(
                                "Unavailable choice",
                                .value = "disabled",
                                .enabled = false,
                            )
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
                            .on_value_change = callback!(
                                [set_volume] | value | {
                                    set_volume.set(value);
                                }
                            ),
                        )
                        FlexView(
                            .class = "field",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            BackendCase(
                                "nestix-native-win32",
                                .replacement = layout! {
                                    Checkbox(
                                        "Enable notifications",
                                        .checked = notifications.clone(),
                                        .on_checked_change = callback!(
                                            [set_notifications] | checked | {
                                                set_notifications.set(checked);
                                            }
                                        ),
                                    )
                                },
                            ) {
                                Text("Enable notifications", .class = "choice")
                                Switch(
                                    .checked = notifications.clone(),
                                    .on_checked_change = callback!(
                                        [set_notifications] | checked | {
                                            set_notifications.set(checked);
                                        }
                                    ),
                                )
                            }
                        }
                        FlexView(
                            .class = "actions",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Button(
                                .title = "Save",
                                .disabled = computed!(
                                    [name] || name.get().trim().is_empty()
                                ),
                                .on_click = callback!(
                                    [
                                        name,
                                        newsletter,
                                        notifications,
                                        density,
                                        country,
                                        volume,
                                        set_status,
                                    ] || {
                                        let country = country
                                            .get()
                                            .unwrap_or_else(|| "not selected".to_string());
                                        set_status.set(format!(
                                            "Saved: name={:?}, newsletter={}, notifications={}, density={}, country={}, volume={:.0}",
                                            name.get(),
                                            newsletter.get(),
                                            notifications.get(),
                                            density.get(),
                                            country,
                                            volume.get(),
                                        ));
                                    }
                                ),
                            )
                            Button(
                                .title = "Reset",
                                .disabled = computed!(
                                    [name, newsletter, notifications, density, country, volume]
                                        || {
                                            name.get().is_empty()
                                                && !newsletter.get()
                                                && notifications.get()
                                                && density.get() == "comfortable"
                                                && country.get().is_none()
                                                && volume.get() == 50.0
                                        }
                                ),
                                .on_click = callback!(
                                    [
                                        set_name,
                                        set_newsletter,
                                        set_notifications,
                                        set_density,
                                        set_country,
                                        set_volume,
                                        set_status
                                    ] || {
                                        set_name.set(String::new());
                                        set_newsletter.set(false);
                                        set_notifications.set(true);
                                        set_density.set("comfortable".to_string());
                                        set_country.set(None);
                                        set_volume.set(50.0);
                                        set_status.set("Form reset.".to_string());
                                    }
                                ),
                            )
                        }
                        Text(status)
                    }
                }
            }
        }
    }
}
