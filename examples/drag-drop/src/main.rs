use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, Color, DragContent, DragDataTypes, DragImage, DragOffer, DragOperation,
    DragOperations, DragReadError, DragSource, DragSourceOutcome, DropEvent, DropTarget, FlexView,
    JustifyContent, RGBColor, Root, Text, Window,
};
use nestix_native_winui::WINUI_BACKEND;

const SAMPLE_IMAGE: &[u8] = include_bytes!("../../assets/sample.jpg");

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND,  },
        ) {
            DragDropExample
        }
    });
}

#[component]
fn DragDropExample() -> Element {
    let hovering = create_state(false);
    let status =
        create_state("Drag the card, or drop files, an image, or text onto it.".to_string());
    let mut content = DragContent::new()
        .with_text("Hello from Nestix")
        .with_image(DragImage::new(SAMPLE_IMAGE, "image/jpeg", "sample.jpg"));
    if let Ok(executable) = std::env::current_exe() {
        content = content.with_files([executable]);
    }

    layout! {
        Root {
            Window(
                .title = "Nestix Drag and Drop",
                .desktop(
                    .width = 620,
                    .height = 420,
                    .on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    }),
                ),
            ) {
                FlexView(
                    .align_items = AlignItems::Center,
                    .justify_content = JustifyContent::Center,
                    .view(.flex_grow = 1.0),
                ) {
                    DropTarget(
                        .accepted_types = DragDataTypes::ALL,
                        .on_enter = callback!(
                            [hovering] |_offer: &DragOffer| {
                                hovering.set(true);
                                Some(DragOperation::Copy)
                            }
                        ),
                        .on_over = callback!(
                            |_offer: &DragOffer| Some(DragOperation::Copy)
                        ),
                        .on_leave = callback!([hovering] || hovering.set(false)),
                        .on_drop = callback!(
                            [hovering, status] |event: DropEvent| {
                                hovering.set(false);
                                read_preferred_drop(event, status.clone());
                            }
                        ),
                    ) {
                        DragSource(
                            .content = content,
                            .allowed_operations = DragOperations::COPY,
                            .on_started = callback!(
                                [status] || {
                                    status.set("Dragging all available representations…".to_string());
                                }
                            ),
                            .on_completed = callback!(
                                [status] | outcome | {
                                    status.set(match outcome {
                                        DragSourceOutcome::Dropped(operation) => {
                                            format!("Drag completed with {operation:?}")
                                        }
                                        DragSourceOutcome::Cancelled => {
                                            "Drag cancelled".to_string()
                                        }
                                    });
                                }
                            ),
                            .on_error = callback!(
                                [status] | error | {
                                    status.set(format!("Could not start drag: {error}"));
                                }
                            ),
                        ) {
                            FlexView(
                                .align_items = AlignItems::Center,
                                .justify_content = JustifyContent::Center,
                                .container(.padding = 32),
                                .view(.width = 430, .height = 210),
                                .bg_color = computed!(
                                    [hovering]
                                        || Some(if hovering.get() {
                                            Color::RGB(RGBColor::from_rgb(210, 235, 255))
                                        } else {
                                            Color::RGB(RGBColor::from_rgb(235, 238, 242))
                                        })
                                ),
                            ) {
                                Text("Drag source + drop target")
                                Text("Publishes a file, UTF-8 text, and an encoded PNG.")
                                Text(computed!([status] || format!("Status: {}", status.get())))
                            }
                        }
                    }
                }
            }
        }
    }
}

fn read_preferred_drop(event: DropEvent, status: nestix::State<String>) {
    let available = event.data.available_types();
    // Shell file drags may also advertise an auxiliary text representation.
    // Prefer CF_HDROP so filesystem entries remain files rather than being
    // interpreted as text containing their paths.
    if available.contains(DragDataTypes::FILES) {
        event.data.read_files(
            callback!([status] |result: Result<Vec<std::path::PathBuf>, DragReadError>| {
                status.set(match result {
                    Ok(files) => format!("Dropped files: {files:?}"),
                    Err(error) => format!("Could not read files: {error}"),
                });
            }),
        );
    } else if available.contains(DragDataTypes::TEXT) {
        event
            .data
            .read_text(callback!([status] |result: Result<String, DragReadError>| {
                status.set(match result {
                    Ok(text) => format!("Dropped text: {text}"),
                    Err(error) => format!("Could not read text: {error}"),
                });
            }));
    } else if available.contains(DragDataTypes::IMAGE) {
        event.data.read_image(
            callback!([status] |result: Result<DragImage, DragReadError>| {
                status.set(match result {
                    Ok(image) => format!(
                        "Dropped {} image ({} bytes)",
                        image.media_type,
                        image.bytes.len()
                    ),
                    Err(error) => format!("Could not read image: {error}"),
                });
            }),
        );
    } else {
        status.set("The drop did not contain a supported representation.".to_string());
    }
}
