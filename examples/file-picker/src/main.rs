use env_logger::Env;
use nestix::{ContextProvider, Element, callback, component, create_state, layout, mount_root};
use nestix_native::{
    AlignItems, BackendContext, Button, FilePicker, FilePickerController, FilePickerFilter,
    FilePickerOutcome, FilePickerRequest, FlexView, Root, Text, Window, default_backend,
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
            FilePickerExample
        }
    });
}

#[component]
fn FilePickerExample() -> Element {
    let picker = FilePickerController::new();
    let status = create_state("Choose an operation".to_string());

    layout! {
        Root {
            Window(.title = "Nestix File Picker", .width = 520, .height = 360) {
                FlexView(
                    .align_items = AlignItems::Start,
                    .container(.padding = 24),
                    .view(.flex_grow = 1.0),
                ) {
                    FilePicker(.controller = picker.clone())
                    Text(status.clone())
                    Button(
                        .title = "Open file",
                        .on_click = picker_callback(
                            picker.clone(),
                            status.clone(),
                            FilePickerRequest::open_file().with_filter(
                                FilePickerFilter::new("Images", ["png", "jpg", "jpeg"]),
                            ),
                        ),
                    )
                    Button(
                        .title = "Open multiple files",
                        .on_click = picker_callback(
                            picker.clone(),
                            status.clone(),
                            FilePickerRequest::open_files(),
                        ),
                    )
                    Button(
                        .title = "Save file",
                        .on_click = picker_callback(
                            picker.clone(),
                            status.clone(),
                            FilePickerRequest::save_file()
                                .with_suggested_name("document.txt")
                                .with_filter(FilePickerFilter::new("Text", ["txt"])),
                        ),
                    )
                    Button(
                        .title = "Select folder",
                        .on_click = picker_callback(
                            picker.clone(),
                            status.clone(),
                            FilePickerRequest::select_folder(),
                        ),
                    )
                }
            }
        }
    }
}

fn picker_callback(
    picker: FilePickerController,
    status: nestix::State<String>,
    request: FilePickerRequest,
) -> nestix::Shared<dyn Fn()> {
    callback!(move || {
        let status_for_completion = status.clone();
        if let Err(error) = picker.open(
            request.clone(),
            callback!(move |result| {
                let message = match result {
                    Ok(FilePickerOutcome::Selected(paths)) => format!("Selected: {paths:?}"),
                    Ok(FilePickerOutcome::Cancelled) => "Cancelled".to_string(),
                    Err(error) => format!("Picker failed: {error}"),
                };
                status_for_completion.set(message);
            }),
        ) {
            status.set(format!("Could not open picker: {error}"));
        }
    })
}
