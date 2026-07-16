use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    FilePickerCallback, FilePickerError, FilePickerMode, FilePickerOpenError, FilePickerOutcome,
    FilePickerPresenter, FilePickerProps, FilePickerRegistration, FilePickerRequest,
    FilePickerResult,
};
use windows_collections::{IVector, IVectorView};
use windows_core::{Error, HSTRING, Interface, RuntimeType};
use windows_future::{AsyncOperationCompletedHandler, IAsyncOperation};

use crate::{
    WindowContext,
    bindings::Microsoft::{
        UI::Dispatching::{DispatcherQueue, DispatcherQueueHandler},
        Windows::Storage::Pickers::{
            FileOpenPicker as NativeOpenPicker, FileSavePicker as NativeSavePicker,
            FolderPicker as NativeFolderPicker, PickFileResult, PickFolderResult,
        },
    },
};

thread_local! {
    static NEXT_CALLBACK_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    static CALLBACKS: RefCell<HashMap<u64, FilePickerCallback>> = RefCell::new(HashMap::new());
}

#[component]
pub fn FilePicker(props: &FilePickerProps, element: &Element) {
    let window = element.context::<WindowContext>().unwrap();
    let registration = Rc::new(RefCell::new(None::<FilePickerRegistration>));
    scoped_effect!(
        element,
        [props.controller, registration, window.window] || {
            registration.borrow_mut().take();
            registration
                .borrow_mut()
                .replace(controller.get().bind(FilePickerPresenter {
                    open: callback!(
                        [window] | request,
                        on_complete | present(&window, request, on_complete)
                    ),
                }));
        }
    );
    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
}

fn present(
    window: &crate::xaml::WindowElement,
    request: FilePickerRequest,
    on_complete: FilePickerCallback,
) -> Result<(), FilePickerOpenError> {
    let window_id = window.window_id().map_err(open_error)?;
    let dispatcher = window.dispatcher_queue().map_err(open_error)?;
    match request.mode {
        FilePickerMode::OpenFile => {
            let picker = NativeOpenPicker::CreateInstance(window_id).map_err(open_error)?;
            configure_open(&picker, &request)?;
            attach(
                picker.PickSingleFileAsync().map_err(open_error)?,
                dispatcher,
                on_complete,
                file_result,
            )
        }
        FilePickerMode::OpenFiles => {
            let picker = NativeOpenPicker::CreateInstance(window_id).map_err(open_error)?;
            configure_open(&picker, &request)?;
            attach(
                picker.PickMultipleFilesAsync().map_err(open_error)?,
                dispatcher,
                on_complete,
                multiple_file_result,
            )
        }
        FilePickerMode::SaveFile => {
            let picker = NativeSavePicker::CreateInstance(window_id).map_err(open_error)?;
            configure_save(&picker, &request)?;
            attach(
                picker.PickSaveFileAsync().map_err(open_error)?,
                dispatcher,
                on_complete,
                file_result,
            )
        }
        FilePickerMode::SelectFolder => {
            let picker = NativeFolderPicker::CreateInstance(window_id).map_err(open_error)?;
            attach(
                picker.PickSingleFolderAsync().map_err(open_error)?,
                dispatcher,
                on_complete,
                folder_result,
            )
        }
    }
}

fn configure_open(
    picker: &NativeOpenPicker,
    request: &FilePickerRequest,
) -> Result<(), FilePickerOpenError> {
    let filters = picker.FileTypeFilter().map_err(open_error)?;
    for extension in flattened_extensions(request) {
        filters
            .Append(&HSTRING::from(extension))
            .map_err(open_error)?;
    }
    Ok(())
}

fn configure_save(
    picker: &NativeSavePicker,
    request: &FilePickerRequest,
) -> Result<(), FilePickerOpenError> {
    if let Some(name) = &request.suggested_name {
        picker
            .SetSuggestedFileName(&HSTRING::from(name))
            .map_err(open_error)?;
    }
    if let Some(directory) = &request.initial_directory {
        picker
            .SetSuggestedFolder(&HSTRING::from(directory.as_os_str()))
            .map_err(open_error)?;
    }

    let choices = picker.FileTypeChoices().map_err(open_error)?;
    for filter in &request.filters {
        let extensions = if filter.is_all_files() {
            vec![HSTRING::from("*.*")]
        } else {
            filter
                .extensions
                .iter()
                .map(|extension| HSTRING::from(format!(".{extension}")))
                .collect()
        };
        choices
            .Insert(&HSTRING::from(&filter.name), &IVector::from(extensions))
            .map_err(open_error)?;
    }
    if let Some(extension) = request
        .filters
        .iter()
        .find_map(|filter| filter.extensions.first())
    {
        picker
            .SetDefaultFileExtension(&HSTRING::from(format!(".{extension}")))
            .map_err(open_error)?;
    }
    Ok(())
}

fn flattened_extensions(request: &FilePickerRequest) -> Vec<String> {
    if request.filters.is_empty() {
        return Vec::new();
    }
    request
        .filters
        .iter()
        .flat_map(|filter| {
            if filter.is_all_files() {
                vec!["*".to_string()]
            } else {
                filter
                    .extensions
                    .iter()
                    .map(|extension| format!(".{extension}"))
                    .collect()
            }
        })
        .collect()
}

fn attach<T, F>(
    operation: IAsyncOperation<T>,
    dispatcher: DispatcherQueue,
    on_complete: FilePickerCallback,
    map: F,
) -> Result<(), FilePickerOpenError>
where
    T: RuntimeType + 'static,
    F: Fn(T) -> FilePickerResult + Send + 'static,
{
    let id = register_callback(on_complete);
    let handler = AsyncOperationCompletedHandler::new(move |operation, _| {
        let native_result = operation
            .as_ref()
            .ok_or_else(|| Error::from_hresult(windows_core::HRESULT(0x8000_4005u32 as i32)))
            .and_then(IAsyncOperation::GetResults);
        let result = map_async_result(native_result, &map);
        let result = Arc::new(Mutex::new(Some(result)));
        let queued_result = result.clone();
        let _queued = dispatcher.TryEnqueue(&DispatcherQueueHandler::new(move || {
            if let Some(result) = queued_result.lock().unwrap().take() {
                complete_callback(id, result);
            }
            Ok(())
        }))?;
        Ok(())
    });
    if let Err(error) = operation.SetCompleted(&handler) {
        discard_callback(id);
        return Err(open_error(error));
    }
    Ok(())
}

fn map_async_result<T>(
    result: windows_core::Result<T>,
    map: impl Fn(T) -> FilePickerResult,
) -> FilePickerResult {
    match result {
        Ok(result) => map(result),
        // Nullable WinRT results are represented by windows-rs as an Error
        // carrying S_OK when the ABI returned a null object. The Windows App
        // SDK pickers use that null result to report cancellation.
        Err(error) if error.code().is_ok() => Ok(FilePickerOutcome::Cancelled),
        Err(error) => Err(backend_error(error)),
    }
}

fn file_result(result: PickFileResult) -> FilePickerResult {
    if Interface::as_raw(&result).is_null() {
        return Ok(FilePickerOutcome::Cancelled);
    }
    path_result(result.Path())
}

fn folder_result(result: PickFolderResult) -> FilePickerResult {
    if Interface::as_raw(&result).is_null() {
        return Ok(FilePickerOutcome::Cancelled);
    }
    path_result(result.Path())
}

fn multiple_file_result(results: IVectorView<PickFileResult>) -> FilePickerResult {
    let count = results.Size().map_err(backend_error)?;
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let result = results.GetAt(index).map_err(backend_error)?;
        if Interface::as_raw(&result).is_null() {
            return Err(FilePickerError::NonFilesystemSelection);
        }
        paths.push(
            result
                .Path()
                .map_err(backend_error)?
                .to_string_lossy()
                .into(),
        );
    }
    selected(paths)
}

fn path_result(path: windows_core::Result<HSTRING>) -> FilePickerResult {
    let path = path.map_err(backend_error)?.to_string_lossy();
    if path.is_empty() {
        Err(FilePickerError::NonFilesystemSelection)
    } else {
        selected(vec![PathBuf::from(path)])
    }
}

fn selected(paths: Vec<PathBuf>) -> FilePickerResult {
    if paths.is_empty() {
        Ok(FilePickerOutcome::Cancelled)
    } else {
        Ok(FilePickerOutcome::Selected(paths))
    }
}

fn register_callback(callback: FilePickerCallback) -> u64 {
    let id = NEXT_CALLBACK_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1).max(1));
        id
    });
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.insert(id, callback);
    });
    id
}

fn complete_callback(id: u64, result: FilePickerResult) {
    let callback = CALLBACKS.with_borrow_mut(|callbacks| callbacks.remove(&id));
    if let Some(callback) = callback {
        callback(result);
    }
}

fn discard_callback(id: u64) {
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.remove(&id);
    });
}

fn backend_error(error: Error) -> FilePickerError {
    FilePickerError::Backend(format!("WinUI file picker failed: {error}"))
}

fn open_error(error: Error) -> FilePickerOpenError {
    FilePickerOpenError::BackendUnavailable(format!("WinUI file picker unavailable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nestix_native_core::{FilePickerFilter, FilePickerRequest};

    #[test]
    fn open_filters_are_normalized_for_winui() {
        let request = FilePickerRequest::open_file()
            .with_filter(FilePickerFilter::new("Images", ["png", "jpg"]))
            .with_filter(FilePickerFilter::all_files("All files"));
        assert_eq!(flattened_extensions(&request), [".png", ".jpg", "*"]);
    }

    #[test]
    fn null_success_result_is_cancellation() {
        let result = map_async_result::<PickFileResult>(
            Err(Error::from_hresult(windows_core::HRESULT(0))),
            file_result,
        );
        assert_eq!(result, Ok(FilePickerOutcome::Cancelled));
    }
}
