use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use nestix::{Element, PropValue, Shared, callback, closure, component, layout, scoped_effect};
use nestix_native_core::{
    DragContent, DragDataType, DragDataTypes, DragFilesCallback, DragImage, DragImageCallback,
    DragModifiers, DragOffer, DragOperation, DragOperations, DragReadError, DragSourceError,
    DragSourceOutcome, DragSourceProps, DragTextCallback, DropDataProvider, DropDataReader,
    DropEvent, DropTargetProps,
};
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows_collections::IVector;
use windows_core::{EventRevoker, HSTRING, Interface, RuntimeType};
use windows_future::{AsyncOperationCompletedHandler, IAsyncOperation};

use crate::{
    bindings::{
        Microsoft::UI::{
            Dispatching::{DispatcherQueue, DispatcherQueueHandler},
            Xaml::{DragEventArgs, DragStartingEventArgs, DropCompletedEventArgs, UIElement},
        },
        Windows::{
            ApplicationModel::DataTransfer::{
                DataPackageOperation, DataPackageView, DragDrop::DragDropModifiers,
                StandardDataFormats,
            },
            Storage::{IStorageItem, StorageFile, Streams::RandomAccessStreamReference},
        },
    },
    xaml::{XamlElement, XamlRealizedRegistration},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
const TEMP_FILE_GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(300);

fn schedule_temp_cleanup(path: PathBuf) {
    std::thread::spawn(move || {
        // Explorer can finish the XAML drop before its shell copy worker opens
        // the StorageFile. Preserve this drag's backing files for that read.
        std::thread::sleep(TEMP_FILE_GRACE_PERIOD);
        let _ = std::fs::remove_dir_all(path);
    });
}

thread_local! {
    static SOURCES: RefCell<HashMap<usize, Vec<Rc<SourceState>>>> = RefCell::new(HashMap::new());
    static TARGETS: RefCell<HashMap<usize, Vec<Rc<TargetState>>>> = RefCell::new(HashMap::new());
    static FILE_CALLBACKS: RefCell<HashMap<u64, DragFilesCallback>> = RefCell::new(HashMap::new());
    static IMAGE_CALLBACKS: RefCell<HashMap<u64, DragImageCallback>> = RefCell::new(HashMap::new());
    static TEXT_CALLBACKS: RefCell<HashMap<u64, DragTextCallback>> = RefCell::new(HashMap::new());
}

fn key(element: &UIElement) -> usize {
    Interface::as_raw(element) as usize
}

fn native_operations(value: DragOperations) -> DataPackageOperation {
    let mut result = DataPackageOperation::None;
    if value.contains(DragOperations::COPY) {
        result |= DataPackageOperation::Copy;
    }
    if value.contains(DragOperations::MOVE) {
        result |= DataPackageOperation::Move;
    }
    if value.contains(DragOperations::LINK) {
        result |= DataPackageOperation::Link;
    }
    result
}

fn operations(value: DataPackageOperation) -> DragOperations {
    let mut result = DragOperations::NONE;
    if value.contains(DataPackageOperation::Copy) {
        result |= DragOperations::COPY;
    }
    if value.contains(DataPackageOperation::Move) {
        result |= DragOperations::MOVE;
    }
    if value.contains(DataPackageOperation::Link) {
        result |= DragOperations::LINK;
    }
    result
}

fn native_operation(value: Option<DragOperation>) -> DataPackageOperation {
    match value {
        Some(DragOperation::Copy) => DataPackageOperation::Copy,
        Some(DragOperation::Move) => DataPackageOperation::Move,
        Some(DragOperation::Link) => DataPackageOperation::Link,
        None => DataPackageOperation::None,
    }
}

fn operation(value: DataPackageOperation) -> Option<DragOperation> {
    if value.contains(DataPackageOperation::Move) {
        Some(DragOperation::Move)
    } else if value.contains(DataPackageOperation::Copy) {
        Some(DragOperation::Copy)
    } else if value.contains(DataPackageOperation::Link) {
        Some(DragOperation::Link)
    } else {
        None
    }
}

fn modifiers(value: DragDropModifiers) -> DragModifiers {
    let mut result = DragModifiers::NONE;
    if value.contains(DragDropModifiers::Control) {
        result |= DragModifiers::PRIMARY;
    }
    if value.contains(DragDropModifiers::Shift) {
        result |= DragModifiers::SHIFT;
    }
    if value.contains(DragDropModifiers::Alt) {
        result |= DragModifiers::ALT;
    }
    result
}

fn available_types(view: &DataPackageView) -> DragDataTypes {
    let mut result = DragDataTypes::NONE;
    if StandardDataFormats::StorageItems()
        .ok()
        .and_then(|f| view.Contains(&f).ok())
        .unwrap_or(false)
    {
        result |= DragDataTypes::FILES;
    }
    if StandardDataFormats::Bitmap()
        .ok()
        .and_then(|f| view.Contains(&f).ok())
        .unwrap_or(false)
    {
        result |= DragDataTypes::IMAGE;
    }
    if StandardDataFormats::Text()
        .ok()
        .and_then(|f| view.Contains(&f).ok())
        .unwrap_or(false)
    {
        result |= DragDataTypes::TEXT;
    }
    result
}

fn dispatch(dispatcher: &DispatcherQueue, action: impl FnOnce() + Send + 'static) {
    let action = Arc::new(Mutex::new(Some(action)));
    let _ = dispatcher.TryEnqueue(&DispatcherQueueHandler::new(move || {
        if let Some(action) = action.lock().unwrap().take() {
            action();
        }
        Ok(())
    }));
}

fn register_files(callback: DragFilesCallback) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    FILE_CALLBACKS.with_borrow_mut(|v| {
        v.insert(id, callback);
    });
    id
}
fn register_image(callback: DragImageCallback) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    IMAGE_CALLBACKS.with_borrow_mut(|v| {
        v.insert(id, callback);
    });
    id
}
fn register_text(callback: DragTextCallback) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    TEXT_CALLBACKS.with_borrow_mut(|v| {
        v.insert(id, callback);
    });
    id
}
fn complete_files(id: u64, result: Result<Vec<PathBuf>, DragReadError>) {
    if let Some(f) = FILE_CALLBACKS.with_borrow_mut(|v| v.remove(&id)) {
        f(result)
    }
}
fn complete_image(id: u64, result: Result<DragImage, DragReadError>) {
    if let Some(f) = IMAGE_CALLBACKS.with_borrow_mut(|v| v.remove(&id)) {
        f(result)
    }
}
fn complete_text(id: u64, result: Result<String, DragReadError>) {
    if let Some(f) = TEXT_CALLBACKS.with_borrow_mut(|v| v.remove(&id)) {
        f(result)
    }
}

fn start_async<T: RuntimeType + 'static>(
    operation: IAsyncOperation<T>,
    work: impl FnOnce(windows_core::Result<T>) + Send + 'static,
) -> windows_core::Result<()> {
    let work = Arc::new(Mutex::new(Some(work)));
    operation.SetCompleted(&AsyncOperationCompletedHandler::new(move |operation, _| {
        let result = operation
            .as_ref()
            .ok_or_else(|| windows_core::Error::empty())
            .and_then(IAsyncOperation::GetResults);
        if let Some(work) = work.lock().unwrap().take() {
            work(result);
        }
        Ok(())
    }))
}

fn reader(
    view: DataPackageView,
    dispatcher: DispatcherQueue,
    types: DragDataTypes,
) -> DropDataReader {
    DropDataReader::new(DropDataProvider {
        available_types: types,
        read_files: callback!([view, dispatcher] |done: DragFilesCallback| {
            let id=register_files(done); let dispatcher=dispatcher.clone();
            let result=view.GetStorageItemsAsync().and_then(|op| start_async(op, move |result| {
                let result=result.map_err(|e|DragReadError::Backend(e.to_string())).and_then(|items| {
                    let size=items.Size().map_err(|e|DragReadError::Backend(e.to_string()))?; let mut paths=Vec::with_capacity(size as usize);
                    for index in 0..size { let item=items.GetAt(index).map_err(|e|DragReadError::Backend(e.to_string()))?; let path=item.Path().map_err(|e|DragReadError::Backend(e.to_string()))?.to_string_lossy(); if !path.is_empty(){paths.push(PathBuf::from(path));} }
                    if paths.is_empty(){Err(DragReadError::Unavailable(DragDataType::Files))}else{Ok(paths)}
                }); dispatch(&dispatcher,move||complete_files(id,result));
            }));
            if let Err(error)=result { complete_files(id,Err(DragReadError::Backend(error.to_string()))); }
        }),
        read_text: callback!([view, dispatcher] |done: DragTextCallback| {
            let id=register_text(done); let dispatcher=dispatcher.clone();
            let result=view.GetTextAsync().and_then(|op| start_async(op,move|result|{let result=result.map(|v|v.to_string_lossy()).map_err(|e|DragReadError::Backend(e.to_string()));dispatch(&dispatcher,move||complete_text(id,result));}));
            if let Err(error)=result { complete_text(id,Err(DragReadError::Backend(error.to_string()))); }
        }),
        read_image: callback!([view, dispatcher] |done: DragImageCallback| {
            let id=register_image(done); let dispatcher=dispatcher.clone();
            let result=view.GetBitmapAsync().and_then(|op| start_async(op,move|result|{
                let result=(|| { let reference=result.map_err(|e|DragReadError::Backend(e.to_string()))?; let stream=reference.OpenReadAsync().map_err(|e|DragReadError::Backend(e.to_string()))?.join().map_err(|e|DragReadError::Backend(e.to_string()))?; let media=stream.ContentType().map_err(|e|DragReadError::Backend(e.to_string()))?.to_string_lossy(); let size=stream.Size().map_err(|e|DragReadError::Backend(e.to_string()))?; let raw=stream.into_raw(); let native=unsafe{windows::Storage::Streams::IRandomAccessStreamWithContentType::from_raw(raw)}; let input=native.GetInputStreamAt(0).map_err(|e|DragReadError::Backend(e.to_string()))?; let reader=DataReader::CreateDataReader(&input).map_err(|e|DragReadError::Backend(e.to_string()))?; let loaded=reader.LoadAsync(size as u32).map_err(|e|DragReadError::Backend(e.to_string()))?.join().map_err(|e|DragReadError::Backend(e.to_string()))?; let mut bytes=vec![0;loaded as usize]; reader.ReadBytes(&mut bytes).map_err(|e|DragReadError::Backend(e.to_string()))?; let extension=match media.as_str(){"image/png"=>"png","image/jpeg"=>"jpg","image/bmp"=>"bmp","image/gif"=>"gif",_=>"img"}; Ok(DragImage::new(bytes,media,format!("image.{extension}"))) })();
                dispatch(&dispatcher,move||complete_image(id,result));
            }));
            if let Err(error)=result { complete_image(id,Err(DragReadError::Backend(error.to_string()))); }
        }),
    })
}

struct TargetState {
    id: u64,
    element: UIElement,
    enabled: PropValue<bool>,
    accepted: PropValue<DragDataTypes>,
    default_operation: PropValue<DragOperation>,
    on_enter: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_over: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_leave: PropValue<Option<Shared<dyn Fn()>>>,
    on_drop: PropValue<Shared<dyn Fn(DropEvent)>>,
    dispatcher: DispatcherQueue,
    current: Cell<Option<DragOperation>>,
}

fn active_target(id: u64) -> Option<Rc<TargetState>> {
    TARGETS.with_borrow(|all| {
        all.values()
            .find_map(|v| v.last().filter(|s| s.id == id).cloned())
    })
}
fn drag_offer(state: &TargetState, args: &DragEventArgs) -> Option<DragOffer> {
    let view = args.DataView().ok()?;
    let point = args.GetPosition(&state.element).ok()?;
    Some(DragOffer {
        available_types: available_types(&view).intersection(state.accepted.get()),
        allowed_operations: operations(args.AllowedOperations().ok()?),
        position: nestix_native_core::dpi::LogicalPosition::new(point.X as f64, point.Y as f64),
        modifiers: modifiers(args.Modifiers().unwrap_or_default()),
    })
}
fn decide(state: &TargetState, args: &DragEventArgs, entering: bool) -> Option<DragOperation> {
    if !state.enabled.get() {
        return None;
    }
    let offer = drag_offer(state, args)?;
    if offer.available_types.is_empty() {
        return None;
    }
    let handler = if entering {
        state.on_enter.get()
    } else {
        state.on_over.get()
    };
    handler
        .as_ref()
        .and_then(|f| f(&offer))
        .or_else(|| handler.is_none().then(|| state.default_operation.get()))
        .filter(|op| offer.allowed_operations.contains_operation(*op))
}
fn target_enter(id: u64, args: &DragEventArgs) {
    if let Some(state) = active_target(id) {
        let op = decide(&state, args, true);
        state.current.set(op);
        let _ = args.SetAcceptedOperation(native_operation(op));
        let _ = args.SetHandled(true);
    }
}
fn target_over(id: u64, args: &DragEventArgs) {
    if let Some(state) = active_target(id) {
        let op = decide(&state, args, false);
        state.current.set(op);
        let _ = args.SetAcceptedOperation(native_operation(op));
        let _ = args.SetHandled(true);
    }
}
fn target_leave(id: u64, args: &DragEventArgs) {
    if let Some(state) = active_target(id) {
        state.current.set(None);
        if let Some(f) = state.on_leave.get() {
            f()
        }
        let _ = args.SetHandled(true);
    }
}
fn target_drop(id: u64, args: &DragEventArgs) {
    if let Some(state) = active_target(id) {
        let Some(offer) = drag_offer(&state, args) else {
            return;
        };
        let op = state
            .current
            .take()
            .filter(|op| offer.allowed_operations.contains_operation(*op));
        let _ = args.SetAcceptedOperation(native_operation(op));
        let _ = args.SetHandled(true);
        if let Some(op) = op {
            if let Ok(view) = args.DataView() {
                (state.on_drop.get())(DropEvent {
                    operation: op,
                    position: offer.position,
                    modifiers: offer.modifiers,
                    data: reader(view, state.dispatcher.clone(), offer.available_types),
                });
            }
        }
    }
}

struct TargetRegistration {
    key: usize,
    state: Rc<TargetState>,
    _events: [EventRevoker; 4],
}
impl Drop for TargetRegistration {
    fn drop(&mut self) {
        let empty = TARGETS.with_borrow_mut(|all| {
            let Some(v) = all.get_mut(&self.key) else {
                return false;
            };
            v.retain(|s| s.id != self.state.id);
            if v.is_empty() {
                all.remove(&self.key);
                true
            } else {
                false
            }
        });
        if empty {
            let _ = self.state.element.SetAllowDrop(false);
        }
    }
}
fn register_target(state: Rc<TargetState>) -> windows_core::Result<TargetRegistration> {
    let key = key(&state.element);
    state.element.SetAllowDrop(true)?;
    let id = state.id;
    let enter = state.element.DragEnter(move |_, args| {
        if let Some(args) = args.as_ref() {
            target_enter(id, args)
        }
    })?;
    let id = state.id;
    let over = state.element.DragOver(move |_, args| {
        if let Some(args) = args.as_ref() {
            target_over(id, args)
        }
    })?;
    let id = state.id;
    let leave = state.element.DragLeave(move |_, args| {
        if let Some(args) = args.as_ref() {
            target_leave(id, args)
        }
    })?;
    let id = state.id;
    let drop = state.element.Drop(move |_, args| {
        if let Some(args) = args.as_ref() {
            target_drop(id, args)
        }
    })?;
    TARGETS.with_borrow_mut(|all| all.entry(key).or_default().push(state.clone()));
    Ok(TargetRegistration {
        key,
        state,
        _events: [enter, over, leave, drop],
    })
}

#[component]
pub fn DropTarget(props: &DropTargetProps, element: &Element) -> Element {
    let registration = Rc::new(RefCell::new(None::<TargetRegistration>));
    let realized_registration = Rc::new(RefCell::new(None::<XamlRealizedRegistration>));
    scoped_effect!(
        element,
        [
            registration,
            realized_registration,
            props.children,
            props.enabled,
            props.accepted_types,
            props.default_operation,
            props.on_enter,
            props.on_over,
            props.on_leave,
            props.on_drop
        ] || {
            registration.borrow_mut().take();
            realized_registration.borrow_mut().take();
            children.get().on_last_handle_change(closure!(
                [
                    registration,
                    realized_registration,
                    enabled,
                    accepted_types,
                    default_operation,
                    on_enter,
                    on_over,
                    on_leave,
                    on_drop
                ] | handle
                    | {
                        registration.borrow_mut().take();
                        realized_registration.borrow_mut().take();
                        let Some(handle) = handle else { return };
                        let Some(control) = handle.downcast_ref::<XamlElement>() else {
                            return;
                        };
                        let result = control.on_realized(callback!(
                            [registration, enabled, accepted_types, default_operation, on_enter, on_over, on_leave, on_drop]
                            |native: UIElement| {
                                let Ok(dispatcher) = DispatcherQueue::GetForCurrentThread() else { return };
                                let state = Rc::new(TargetState {
                                    id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                                    element: native,
                                    enabled: enabled.clone(),
                                    accepted: accepted_types.clone(),
                                    default_operation: default_operation.clone(),
                                    on_enter: on_enter.clone(),
                                    on_over: on_over.clone(),
                                    on_leave: on_leave.clone(),
                                    on_drop: on_drop.clone(),
                                    dispatcher,
                                    current: Cell::new(None),
                                });
                                if let Ok(value) = register_target(state) {
                                    registration.borrow_mut().replace(value);
                                }
                            }
                        ));
                        if let Ok(result) = result {
                            realized_registration.borrow_mut().replace(result);
                        }
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration, realized_registration] || {
            registration.borrow_mut().take();
            realized_registration.borrow_mut().take();
        }
    ));
    layout! {$(props.children.get())}
}

struct SourceState {
    id: u64,
    element: UIElement,
    content: PropValue<DragContent>,
    enabled: PropValue<bool>,
    allowed: PropValue<DragOperations>,
    on_started: PropValue<Option<Shared<dyn Fn()>>>,
    on_completed: PropValue<Option<Shared<dyn Fn(DragSourceOutcome)>>>,
    on_error: PropValue<Option<Shared<dyn Fn(DragSourceError)>>>,
    dispatcher: DispatcherQueue,
    temp_dir: RefCell<Option<PathBuf>>,
}
fn active_source(id: u64) -> Option<Rc<SourceState>> {
    SOURCES.with_borrow(|all| {
        all.values()
            .find_map(|v| v.last().filter(|s| s.id == id).cloned())
    })
}
fn report_source_error(id: u64, message: String) {
    if let Some(state) = active_source(id) {
        if let Some(f) = state.on_error.get() {
            f(DragSourceError::Backend(message));
        }
    }
}
fn image_reference(image: &DragImage) -> windows_core::Result<RandomAccessStreamReference> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(&image.bytes)?;
    writer.StoreAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    let native: windows::Storage::Streams::IRandomAccessStream = stream.cast()?;
    let raw = native.into_raw();
    let custom =
        unsafe { crate::bindings::Windows::Storage::Streams::IRandomAccessStream::from_raw(raw) };
    RandomAccessStreamReference::CreateFromStream(&custom)
}
fn source_start(id: u64, args: &DragStartingEventArgs) {
    let Some(state) = active_source(id) else {
        return;
    };
    if !state.enabled.get() {
        let _ = args.SetCancel(true);
        return;
    }
    let content = state.content.get();
    if content.is_empty() {
        let _ = args.SetCancel(true);
        if let Some(f) = state.on_error.get() {
            f(DragSourceError::EmptyContent)
        }
        return;
    }
    let data = match args.Data() {
        Ok(v) => v,
        Err(e) => {
            report_source_error(id, e.to_string());
            return;
        }
    };
    if let Some(text) = content.text() {
        let _ = data.SetText(&HSTRING::from(text));
    }
    if let Some(image) = content.image() {
        match image_reference(image).and_then(|v| data.SetBitmap(&v)) {
            Ok(()) => {}
            Err(e) => {
                let _ = args.SetCancel(true);
                report_source_error(id, e.to_string());
                return;
            }
        }
    }
    let mut paths: Vec<PathBuf> = content.files().unwrap_or_default().to_vec();
    let drag_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("nestix-drag-{}-{drag_id}", std::process::id()));
    if content.image().is_some() || content.text().is_some() {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let _ = args.SetCancel(true);
            report_source_error(id, e.to_string());
            return;
        }
        if let Some(image) = content.image() {
            let path = dir.join(&image.suggested_name);
            if let Err(e) = std::fs::write(&path, &image.bytes) {
                let _ = args.SetCancel(true);
                report_source_error(id, e.to_string());
                return;
            }
            paths.push(path);
        }
        if let Some(text) = content.text() {
            let path = dir.join("nestix.txt");
            if let Err(e) = std::fs::write(&path, text.as_bytes()) {
                let _ = args.SetCancel(true);
                report_source_error(id, e.to_string());
                return;
            }
            paths.push(path);
        }
        state.temp_dir.replace(Some(dir));
    }
    let _ = args.SetAllowedOperations(native_operations(state.allowed.get()));
    if let Some(f) = state.on_started.get() {
        f()
    }
    if paths.is_empty() {
        return;
    }
    let deferral = match args.GetDeferral() {
        Ok(v) => v,
        Err(e) => {
            let _ = args.SetCancel(true);
            report_source_error(id, e.to_string());
            return;
        }
    };
    let args = args.clone();
    let dispatcher = state.dispatcher.clone();
    std::thread::spawn(move || {
        let result = (|| {
            let mut items = Vec::with_capacity(paths.len());
            for path in paths {
                let file =
                    StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.join()?;
                items.push(Some(file.cast::<IStorageItem>()?));
            }
            let values = IVector::from(items);
            data.SetStorageItems(&values, true)
        })();
        if let Err(error) = result {
            let _ = args.SetCancel(true);
            dispatch(&dispatcher, move || {
                report_source_error(id, error.to_string())
            });
        }
        let _ = deferral.Complete();
    });
}
fn source_completed(id: u64, args: &DropCompletedEventArgs) {
    if let Some(state) = active_source(id) {
        if let Some(dir) = state.temp_dir.take() {
            schedule_temp_cleanup(dir);
        }
        if let Some(f) = state.on_completed.get() {
            f(operation(args.DropResult().unwrap_or_default())
                .map(DragSourceOutcome::Dropped)
                .unwrap_or(DragSourceOutcome::Cancelled));
        }
    }
}
struct SourceRegistration {
    key: usize,
    state: Rc<SourceState>,
    _events: [EventRevoker; 2],
}
impl Drop for SourceRegistration {
    fn drop(&mut self) {
        let empty = SOURCES.with_borrow_mut(|all| {
            let Some(v) = all.get_mut(&self.key) else {
                return false;
            };
            v.retain(|s| s.id != self.state.id);
            if v.is_empty() {
                all.remove(&self.key);
                true
            } else {
                false
            }
        });
        if let Some(dir) = self.state.temp_dir.take() {
            schedule_temp_cleanup(dir);
        }
        if empty {
            let _ = self.state.element.SetCanDrag(false);
        }
    }
}
fn register_source(state: Rc<SourceState>) -> windows_core::Result<SourceRegistration> {
    let key = key(&state.element);
    state.element.SetCanDrag(true)?;
    let id = state.id;
    let start = state.element.DragStarting(move |_, args| {
        if let Some(args) = args.as_ref() {
            source_start(id, args)
        }
    })?;
    let id = state.id;
    let completed = state.element.DropCompleted(move |_, args| {
        if let Some(args) = args.as_ref() {
            source_completed(id, args)
        }
    })?;
    SOURCES.with_borrow_mut(|all| all.entry(key).or_default().push(state.clone()));
    Ok(SourceRegistration {
        key,
        state,
        _events: [start, completed],
    })
}

#[component]
pub fn DragSource(props: &DragSourceProps, element: &Element) -> Element {
    let registration = Rc::new(RefCell::new(None::<SourceRegistration>));
    let realized_registration = Rc::new(RefCell::new(None::<XamlRealizedRegistration>));
    scoped_effect!(
        element,
        [
            registration,
            realized_registration,
            props.children,
            props.content,
            props.enabled,
            props.allowed_operations,
            props.on_started,
            props.on_completed,
            props.on_error
        ] || {
            registration.borrow_mut().take();
            realized_registration.borrow_mut().take();
            children.get().on_last_handle_change(closure!(
                [
                    registration,
                    realized_registration,
                    content,
                    enabled,
                    allowed_operations,
                    on_started,
                    on_completed,
                    on_error
                ] | handle
                    | {
                        registration.borrow_mut().take();
                        realized_registration.borrow_mut().take();
                        let Some(handle) = handle else { return };
                        let Some(control) = handle.downcast_ref::<XamlElement>() else {
                            return;
                        };
                        let result = control.on_realized(callback!(
                            [registration, content, enabled, allowed_operations, on_started, on_completed, on_error]
                            |native: UIElement| {
                                let Ok(dispatcher) = DispatcherQueue::GetForCurrentThread() else { return };
                                let state = Rc::new(SourceState {
                                    id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                                    element: native,
                                    content: content.clone(),
                                    enabled: enabled.clone(),
                                    allowed: allowed_operations.clone(),
                                    on_started: on_started.clone(),
                                    on_completed: on_completed.clone(),
                                    on_error: on_error.clone(),
                                    dispatcher,
                                    temp_dir: RefCell::new(None),
                                });
                                if let Ok(value) = register_source(state) {
                                    registration.borrow_mut().replace(value);
                                }
                            }
                        ));
                        if let Ok(result) = result {
                            realized_registration.borrow_mut().replace(result);
                        }
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration, realized_registration] || {
            registration.borrow_mut().take();
            realized_registration.borrow_mut().take();
        }
    ));
    layout! {$(props.children.get())}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_mapping_round_trips() {
        for op in [
            DragOperation::Copy,
            DragOperation::Move,
            DragOperation::Link,
        ] {
            assert_eq!(operation(native_operation(Some(op))), Some(op));
        }
    }
}
