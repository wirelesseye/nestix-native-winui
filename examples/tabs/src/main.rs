use std::rc::Rc;

use env_logger::Env;
use nestix::{
    ContextProvider, Element, Shared, callback, component, computed, create_state, destructure,
    layout, mount_root, props,
};
use nestix_native::{
    AlignItems, BackendContext, Button, Color, FlexDirection, FlexView, Input, RGBColor, Root,
    ScrollView, StyleProvider, TabView, TabViewItem, Text, Window, default_backend, style,
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

fn random_color() -> Color {
    let r = rand::random();
    let g = rand::random();
    let b = rand::random();
    Color::RGB(RGBColor::from_rgb(r, g, b))
}

#[component]
fn ExampleApp() -> Element {
    let styles = style! {
        .app {
            // bg_color: #F4F6F8;
        }

        .counter {
            padding: 10px;
        }

        .counter > .__Text, .counter > .__Button {
            margin_bottom: 12px;
        }

        .toolbar {
            margin_bottom: 12px;
        }

        .toolbar > .__Input {
            margin_right: 8px;
        }

        .todo_item {
            padding_horizontal: 10px;
            padding_vertical: 8px;
            gap: 6px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix Tabs",
                    .width = 520,
                    .height = 420,
                ) {
                    FlexView(.class = "app", .view(.grow = 1.0)) {
                        TabView(.view(.grow = 1.0)) {
                            TabViewItem(
                                .id = "counter",
                                .title = "Counter",
                            ) { Counter }
                            TabViewItem(
                                .id = "todo_list",
                                .title = "Todo List",
                            ) { TodoList }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Counter() -> Element {
    let count = create_state(0);
    let bg_color = create_state(Color::TRANSPARENT);
    let styles = computed!(
        [bg_color]
            || style! {
                .counter {
                    bg_color: $(bg_color.get());
                }
            }
    );

    layout! {
        StyleProvider(styles) {
            FlexView(.class = "counter") {
                Text(computed!([count] || format!("Count: {}", count.get())))
                Button(
                    .title = "Increment",
                    .on_click = callback!([count] || {
                        count.mutate(|count| *count += 1);
                        bg_color.set(random_color());
                    }),
                )
                if count.get() % 2 == 0 {
                    Text("The count is even")
                }
            }
        }
    }
}

#[component]
fn TodoList() -> Element {
    let items = create_state::<Vec<(String, String)>>(Vec::new());
    let input_text = create_state("".to_string());

    let on_text_change = callback!([input_text] |text: &str| {
        input_text.set(text.to_string());
    });

    let add = callback!(
        [items, input_text] || {
            let text = input_text.get();
            if !text.is_empty() {
                items.mutate(|items| {
                    items.push((nanoid::nanoid!(), text));
                });
                input_text.set("".to_string());
            }
        }
    );

    let remove = callback!([items] |key: &str| {
        items.mutate(|items| {
            items.retain(|(k, _)| k != key);
        });
    });

    let move_up = callback!([items] |key: &str| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                if index > 0 {
                    items.swap(index, index - 1);
                }
            }
        });
    });

    let move_down = callback!([items] |key: &str| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                if index < items.len() - 1 {
                    items.swap(index, index + 1);
                }
            }
        });
    });

    let set_content = callback!([items] |key: &str, content: String| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                items[index] = (key.to_string(), content);
            }
        });
    });

    layout! {
        FlexView(.class = "todo") {
            FlexView(
                .class = "toolbar",
                .flex_direction = FlexDirection::Row,
                .align_items = AlignItems::Center,
            ) {
                Input(
                    .view(.grow = 1.0)
                    .value = input_text,
                    .on_text_change = on_text_change,
                )
                Button(.title = "Add", .on_click = add)
            }
            ScrollView(.view(.grow = 1.0)) {
                FlexView(.view(.grow = 1.0)) {
                    for item in items where key = |item| item.0.clone() {
                        TodoListItem(
                            .data = item,
                            .remove = remove.clone(),
                            .move_up = move_up.clone(),
                            .move_down = move_down.clone(),
                            .set_content = set_content.clone(),
                        )
                    }
                }
            }
        }
    }
}

#[props]
struct TodoListItemProps {
    data: (String, String),

    // Use `raw` because we know these props will never be reactive
    #[props(raw)]
    remove: Shared<dyn Fn(&str)>,
    #[props(raw)]
    move_up: Shared<dyn Fn(&str)>,
    #[props(raw)]
    move_down: Shared<dyn Fn(&str)>,
    #[props(raw)]
    set_content: Shared<dyn Fn(&str, String)>,
}

#[component]
fn TodoListItem(props: &TodoListItemProps) -> Element {
    let is_edit = create_state(false);

    let toggle_edit = callback!(
        [is_edit] || {
            is_edit.update(|is_edit| !is_edit);
        }
    );

    destructure!((key, value) <- props.data);

    layout! {
        FlexView(.class = "todo_item", .flex_direction = FlexDirection::Row, .align_items = AlignItems::Center) {
            if is_edit.get() {
                Input(
                    .value = value.clone(),
                    .on_text_change = callback!([key, props.set_content] |value: &str| {
                        set_content(&key.get(), value.to_string());
                    }),
                    .view(.grow = 1.0),
                )
            } else {
                Text(value.clone(), .view(.grow = 1.0))
            }

            Button(
                .title = "Delete",
                .on_click = callback!([key, props.remove] || remove(&key.get()))
            )
            Button(
                .title = "Up",
                .on_click = callback!([key, props.move_up] || move_up(&key.get()))
            )
            Button(
                .title = "Down",
                .on_click = callback!([key, props.move_down] || move_down(&key.get()))
            )
            Button(
                .title = "Edit",
                .on_click = toggle_edit
            )
        }
    }
}
