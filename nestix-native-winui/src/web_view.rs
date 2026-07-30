use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Component, Path, PathBuf},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    JavaScriptEvaluator, StyleContext, WebViewBridge, WebViewBridgeScriptContext,
    WebViewDevToolsError, WebViewPresenter, WebViewProps, WebViewRegistration, WebViewSource,
    matched_style, resolved_view_style,
};
use windows_core::{EventRevoker, HSTRING, Interface};
use windows_future::{
    AsyncActionCompletedHandler, AsyncOperationCompletedHandler, IAsyncAction, IAsyncOperation,
};

use crate::{
    bindings::{
        Microsoft::{
            UI::Xaml::{
                Controls::{Canvas, WebView2},
                HorizontalAlignment, UIElement, VerticalAlignment,
            },
            Web::WebView2::Core::{CoreWebView2, CoreWebView2HostResourceAccessKind},
        },
        Windows::UI::Color,
    },
    native_control,
    xaml::CanvasElement,
};

const RESOURCE_HOST_PREFIX: &str = "nestix-resource-";
static NEXT_RESOURCE_HOST: AtomicU64 = AtomicU64::new(1);
static NEXT_WEB_VIEW_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static WEB_VIEW_STATES: RefCell<HashMap<u64, Weak<RefCell<WebViewState>>>> =
        RefCell::new(HashMap::new());
}

struct WebViewState {
    mounted: bool,
    source: WebViewSource,
    transparent: bool,
    inspectable: bool,
    bridge: Option<Rc<dyn WebViewBridge>>,
    control: Option<WebView2>,
    core: Option<CoreWebView2>,
    initialization_error: Option<String>,
    ready_to_navigate: bool,
    loaded_revoker: Option<EventRevoker>,
    initialized_revoker: Option<EventRevoker>,
    size_revoker: Option<EventRevoker>,
    message_revoker: Option<EventRevoker>,
    initialization_action: Option<IAsyncAction>,
    script_action: Option<IAsyncOperation<HSTRING>>,
    script_id: Option<HSTRING>,
    evaluation_actions: Vec<IAsyncOperation<HSTRING>>,
}

/// Displays web content in the WinUI WebView2 XAML control.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) {
    require_visual_mount!(element, WebView);
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__winui_WebView"];

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style = resolved_view_style(matched, &props.view);
    let bridge = props.bridge.get();
    let state = Rc::new(RefCell::new(WebViewState {
        mounted: true,
        source: props.source.get(),
        transparent: props.transparent.get(),
        inspectable: props.inspectable.get(),
        bridge,
        control: None,
        core: None,
        initialization_error: None,
        ready_to_navigate: false,
        loaded_revoker: None,
        initialized_revoker: None,
        size_revoker: None,
        message_revoker: None,
        initialization_action: None,
        script_action: None,
        script_id: None,
        evaluation_actions: Vec::new(),
    }));
    let state_id = register_state(&state);

    let canvas = CanvasElement::new().expect("failed to create WebView host canvas");
    let realized_registration = Rc::new(RefCell::new(Some(
        canvas
            .on_realized(callback!([state_id] |element: UIElement| {
            realize_web_view(state_id, &element)
                .expect("failed to realize WinUI WebView2");
            }))
            .expect("failed to observe WebView host realization"),
    )));

    native_control::mount_with_intrinsic_size(
        element,
        canvas.erased(),
        style,
        &props.view,
        (300.0, 150.0),
    );

    scoped_effect!(
        [state, props.source] || {
            let source = source.get();
            let core = {
                let mut state = state.borrow_mut();
                state.source = source.clone();
                state
                    .ready_to_navigate
                    .then(|| state.core.clone())
                    .flatten()
            };
            if let Some(core) = core {
                load_source(&core, source);
            }
        }
    );
    scoped_effect!(
        [state, props.transparent] || {
            let transparent = transparent.get();
            let control = {
                let mut state = state.borrow_mut();
                state.transparent = transparent;
                state.control.clone()
            };
            if let Some(control) = control {
                apply_transparency(&control, transparent);
            }
        }
    );
    scoped_effect!(
        [state, props.inspectable] || {
            let inspectable = inspectable.get();
            let core = {
                let mut state = state.borrow_mut();
                state.inspectable = inspectable;
                state.core.clone()
            };
            if let Some(core) = core {
                apply_inspectable(&core, inspectable);
            }
        }
    );

    let controller_registration = Rc::new(RefCell::new(None::<WebViewRegistration>));
    scoped_effect!(
        [
            state,
            props.inspectable,
            props.controller,
            controller_registration
        ] || {
            controller_registration.borrow_mut().take();
            let weak_state = Rc::downgrade(&state);
            controller_registration
                .borrow_mut()
                .replace(controller.get().bind(WebViewPresenter {
                    open_dev_tools: callback!(
                        [weak_state, inspectable] || {
                            if !inspectable.get() {
                                return Err(WebViewDevToolsError::NotInspectable);
                            }
                            let state = weak_state
                                .upgrade()
                                .ok_or(WebViewDevToolsError::NotMounted)?;
                            let state = state.borrow();
                            let core = state.core.clone().ok_or_else(|| {
                                state
                                    .initialization_error
                                    .as_ref()
                                    .map_or(WebViewDevToolsError::NotMounted, |error| {
                                        WebViewDevToolsError::Backend(error.clone())
                                    })
                            })?;
                            core.OpenDevToolsWindow()
                                .map_err(|error| WebViewDevToolsError::Backend(error.to_string()))
                        }
                    ),
                }));
        }
    );

    element.on_unmount(closure!(
        [state, controller_registration] || {
            controller_registration.borrow_mut().take();
            let mut state = state.borrow_mut();
            state.mounted = false;
            state.loaded_revoker.take();
            state.initialized_revoker.take();
            state.size_revoker.take();
            state.message_revoker.take();
            state.initialization_action.take();
            state.script_action.take();
            state.evaluation_actions.clear();
            if let Some(core) = state.core.take() {
                if let Some(script_id) = state.script_id.take() {
                    let _ = core.RemoveScriptToExecuteOnDocumentCreated(&script_id);
                }
            }
            if let Some(control) = state.control.take() {
                let _ = control.Close();
            }
            if let Some(bridge) = &state.bridge {
                bridge.detach();
            }
            WEB_VIEW_STATES.with(|states| states.borrow_mut().remove(&state_id));
            realized_registration.borrow_mut().take();
        }
    ));
}

fn register_state(state: &Rc<RefCell<WebViewState>>) -> u64 {
    let id = NEXT_WEB_VIEW_ID.fetch_add(1, Ordering::Relaxed);
    WEB_VIEW_STATES.with(|states| {
        states.borrow_mut().insert(id, Rc::downgrade(state));
    });
    id
}

fn with_state(result_id: u64, callback: impl FnOnce(&Rc<RefCell<WebViewState>>)) {
    WEB_VIEW_STATES.with(|states| {
        if let Some(state) = states.borrow().get(&result_id).and_then(Weak::upgrade) {
            callback(&state);
        }
    });
}

fn realize_web_view(state_id: u64, host_element: &UIElement) -> windows_core::Result<()> {
    let canvas: Canvas = host_element.cast()?;
    let control = WebView2::new()?;
    let dispatcher = control.DispatcherQueue()?;
    control.SetHorizontalAlignment(HorizontalAlignment::Stretch)?;
    control.SetVerticalAlignment(VerticalAlignment::Stretch)?;
    control.SetWidth(canvas.ActualWidth()?)?;
    control.SetHeight(canvas.ActualHeight()?)?;

    let size_revoker = canvas.SizeChanged(move |_, args| {
        if let Some(args) = &*args {
            let size = args.NewSize().ok();
            with_state(state_id, |state| {
                if let (Some(size), Some(control)) = (size, state.borrow().control.clone()) {
                    let _ = control.SetWidth(size.Width as f64);
                    let _ = control.SetHeight(size.Height as f64);
                }
            });
        }
    })?;

    let initialized_dispatcher = dispatcher.clone();
    let initialized_revoker = control.CoreWebView2Initialized(move |_, args| {
        let error = args.as_ref().and_then(|args| args.Exception().ok());
        if let Some(error) = error.filter(|error| error.is_err()) {
            with_state(state_id, |state| {
                state.borrow_mut().initialization_error = Some(format!(
                    "WinUI WebView2 initialization failed with {error:?}"
                ));
            });
            return;
        }
        finish_web_view_initialization(state_id, &initialized_dispatcher)
            .expect("failed to configure WinUI WebView2");
    })?;
    let ensure_dispatcher = dispatcher.clone();
    let loaded_revoker = control.Loaded(move |_, _| {
        with_state(state_id, |state| {
            let mut state = state.borrow_mut();
            if state.initialization_action.is_none()
                && let Some(control) = state.control.clone()
            {
                let action = control
                    .EnsureCoreWebView2Async()
                    .expect("failed to start WinUI WebView2 initialization");
                let completion_dispatcher = ensure_dispatcher.clone();
                action
                    .SetCompleted(&AsyncActionCompletedHandler::new(move |action, _| {
                        if let Some(action) = &*action
                            && let Err(error) = action.GetResults()
                        {
                            let message = error.to_string();
                            completion_dispatcher.TryEnqueue(
                                &crate::bindings::Microsoft::UI::Dispatching::DispatcherQueueHandler::new(
                                    move || {
                                        with_state(state_id, |state| {
                                            state.borrow_mut().initialization_error =
                                                Some(message.clone());
                                        });
                                        Ok(())
                                    },
                                ),
                            )?;
                        }
                        Ok(())
                    }))
                    .expect("failed to observe WinUI WebView2 initialization");
                state.initialization_action = Some(action);
            }
        });
    })?;

    apply_transparency(
        &control,
        with_state_value(state_id, |state| state.transparent),
    );
    with_state(state_id, |state| {
        let mut state = state.borrow_mut();
        state.control = Some(control.clone());
        state.size_revoker = Some(size_revoker);
        state.initialized_revoker = Some(initialized_revoker);
        state.loaded_revoker = Some(loaded_revoker);
    });
    canvas.Children()?.Append(&control.cast::<UIElement>()?)?;
    if control.IsLoaded()? {
        with_state(state_id, |state| {
            let mut state = state.borrow_mut();
            if state.initialization_action.is_none() {
                state.initialization_action = Some(
                    control
                        .EnsureCoreWebView2Async()
                        .expect("failed to start WinUI WebView2 initialization"),
                );
            }
        });
    }
    Ok(())
}

fn with_state_value<T: Default>(state_id: u64, callback: impl FnOnce(&WebViewState) -> T) -> T {
    WEB_VIEW_STATES.with(|states| {
        states
            .borrow()
            .get(&state_id)
            .and_then(Weak::upgrade)
            .map(|state| callback(&state.borrow()))
            .unwrap_or_default()
    })
}

fn finish_web_view_initialization(
    state_id: u64,
    dispatcher: &crate::bindings::Microsoft::UI::Dispatching::DispatcherQueue,
) -> windows_core::Result<()> {
    let (control, bridge, inspectable) = with_state_value(state_id, |state| {
        (
            state.control.clone(),
            state.bridge.clone(),
            state.inspectable,
        )
    });
    let Some(control) = control else {
        return Ok(());
    };
    let core = control.CoreWebView2()?;
    apply_inspectable(&core, inspectable);

    if bridge.is_some() {
        let message_dispatcher = dispatcher.clone();
        let message_revoker = core.WebMessageReceived(move |_, args| {
            if let Some(args) = &*args
                && let Ok(message) = args.TryGetWebMessageAsString()
            {
                let message = message.to_string_lossy();
                let _ = message_dispatcher.TryEnqueue(
                    &crate::bindings::Microsoft::UI::Dispatching::DispatcherQueueHandler::new(
                        move || {
                            with_state(state_id, |state| {
                                let bridge = state.borrow().bridge.clone();
                                if let Some(bridge) = bridge {
                                    bridge.receive_message(&message);
                                }
                            });
                            Ok(())
                        },
                    ),
                );
            }
        })?;
        with_state(state_id, |state| {
            state.borrow_mut().message_revoker = Some(message_revoker);
        });
    }
    with_state(state_id, |state| {
        state.borrow_mut().core = Some(core.clone())
    });

    let script = bridge.as_ref().and_then(|bridge| {
        bridge.initialization_script(WebViewBridgeScriptContext {
            post_message_expression: "(message) => window.chrome.webview.postMessage(message)",
        })
    });
    if let Some(script) = script {
        let operation = core.AddScriptToExecuteOnDocumentCreatedAsync(&HSTRING::from(script))?;
        let completion_dispatcher = dispatcher.clone();
        let handler = AsyncOperationCompletedHandler::new(move |operation, _| {
            let result = operation
                .as_ref()
                .ok_or_else(|| {
                    windows_core::Error::from_hresult(windows_core::HRESULT(0x80004005u32 as i32))
                })
                .and_then(IAsyncOperation::GetResults);
            let queued_result = result.clone();
            completion_dispatcher.TryEnqueue(
                &crate::bindings::Microsoft::UI::Dispatching::DispatcherQueueHandler::new(
                    move || {
                        complete_initialization_script(state_id, queued_result.clone());
                        Ok(())
                    },
                ),
            )?;
            Ok(())
        });
        operation.SetCompleted(&handler)?;
        with_state(state_id, |state| {
            state.borrow_mut().script_action = Some(operation)
        });
    } else {
        complete_initialization_script(state_id, Ok(HSTRING::new()));
    }
    Ok(())
}

fn complete_initialization_script(state_id: u64, result: windows_core::Result<HSTRING>) {
    let script_id = result.expect("failed to install WebView document-start script");
    with_state(state_id, |state| {
        let (bridge, core, source) = {
            let mut state = state.borrow_mut();
            if !state.mounted {
                return;
            }
            state.script_action.take();
            if !script_id.is_empty() {
                state.script_id = Some(script_id);
            }
            state.ready_to_navigate = true;
            (
                state.bridge.clone(),
                state.core.clone(),
                state.source.clone(),
            )
        };
        let Some(core) = core else {
            return;
        };
        if let Some(bridge) = bridge {
            let weak_state = Rc::downgrade(state);
            let evaluator: JavaScriptEvaluator = Rc::new(move |script| {
                let Some(state) = weak_state.upgrade() else {
                    return;
                };
                let core = state.borrow().core.clone();
                if let Some(core) = core {
                    let operation = core
                        .ExecuteScriptAsync(&HSTRING::from(script))
                        .expect("failed to execute WebView JavaScript");
                    state.borrow_mut().evaluation_actions.push(operation);
                }
            });
            bridge.attach(evaluator);
        }
        load_source(&core, source);
    });
}

fn apply_transparency(control: &WebView2, transparent: bool) {
    let color = if transparent {
        Color {
            A: 0,
            R: 0,
            G: 0,
            B: 0,
        }
    } else {
        Color {
            A: 255,
            R: 255,
            G: 255,
            B: 255,
        }
    };
    control
        .SetDefaultBackgroundColor(color)
        .expect("failed to update WebView2 background");
}

fn apply_inspectable(core: &CoreWebView2, inspectable: bool) {
    core.Settings()
        .and_then(|settings| settings.SetAreDevToolsEnabled(inspectable))
        .expect("failed to update WebView2 developer tools");
}

fn load_source(core: &CoreWebView2, source: WebViewSource) {
    match source {
        WebViewSource::Url(url) => core
            .Navigate(&HSTRING::from(url))
            .expect("WebView2 failed to navigate"),
        WebViewSource::Html { html, base_url } => {
            let html = base_url.map_or(html.clone(), |base| inject_base_url(&html, &base));
            core.NavigateToString(&HSTRING::from(html))
                .expect("WebView2 failed to load HTML");
        }
        WebViewSource::Resource {
            path,
            development_path,
        } => {
            let document = resolve_document_resource(&path, development_path.as_deref());
            let root = document
                .parent()
                .expect("WebView resource must have a parent");
            let host = format!(
                "{RESOURCE_HOST_PREFIX}{}.local",
                NEXT_RESOURCE_HOST.fetch_add(1, Ordering::Relaxed)
            );
            core.SetVirtualHostNameToFolderMapping(
                &HSTRING::from(&host),
                &HSTRING::from(root.as_os_str()),
                CoreWebView2HostResourceAccessKind::Allow,
            )
            .expect("failed to map WebView resource directory");
            let file =
                percent_encode_path(document.file_name().unwrap().to_string_lossy().as_ref());
            core.Navigate(&HSTRING::from(format!("https://{host}/{file}")))
                .expect("WebView2 failed to navigate to resource");
        }
    }
}

fn inject_base_url(html: &str, base_url: &str) -> String {
    let escaped = base_url
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let base = format!(r#"<base href="{escaped}">"#);
    if let Some(index) = html.to_ascii_lowercase().find("<head")
        && let Some(end) = html[index..].find('>')
    {
        let insertion = index + end + 1;
        return format!("{}{}{}", &html[..insertion], base, &html[insertion..]);
    }
    format!("{base}{html}")
}

fn resolve_document_resource(path: &Path, development_path: Option<&Path>) -> PathBuf {
    validate_resource_path(path);
    let package_root = current_package_path();
    let packaged = package_root.as_ref().map(|root| root.join(path));
    if let Some(candidate) = &packaged
        && let Ok(candidate) = candidate.canonicalize()
        && candidate.is_file()
    {
        return candidate;
    }
    if let Some(candidate) = development_path
        && let Ok(candidate) = candidate.canonicalize()
        && candidate.is_file()
    {
        return candidate;
    }
    panic!(
        "WebView resource {path:?} was not found; packaged location: {}; development location: {}",
        packaged.as_ref().map_or_else(
            || "<application is unpackaged>".into(),
            |path| format!("{path:?}")
        ),
        development_path.map_or_else(|| "<not provided>".into(), |path| format!("{path:?}")),
    );
}

fn validate_resource_path(path: &Path) {
    assert!(
        !path.as_os_str().is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "WebView resource paths must be non-empty relative paths without `..`: {path:?}"
    );
}

fn percent_encode_path(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn current_package_path() -> Option<PathBuf> {
    const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;
    unsafe extern "system" {
        fn GetCurrentPackagePath(path_length: *mut u32, path: windows_core::PWSTR) -> i32;
    }
    unsafe {
        let mut length = 0;
        let result = GetCurrentPackagePath(&mut length, windows_core::PWSTR::null());
        if result == APPMODEL_ERROR_NO_PACKAGE {
            return None;
        }
        assert_eq!(
            result, 122,
            "GetCurrentPackagePath size query failed with error {result}"
        );
        let mut buffer = vec![0u16; length as usize];
        let result = GetCurrentPackagePath(&mut length, windows_core::PWSTR(buffer.as_mut_ptr()));
        assert_eq!(
            result, 0,
            "GetCurrentPackagePath failed with error {result}"
        );
        buffer.truncate(
            buffer
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(buffer.len()),
        );
        Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_paths_reject_traversal() {
        assert!(
            std::panic::catch_unwind(|| validate_resource_path(Path::new("../index.html")))
                .is_err()
        );
        assert!(std::panic::catch_unwind(|| validate_resource_path(Path::new(""))).is_err());
    }

    #[test]
    fn html_base_url_is_inserted_in_head() {
        assert_eq!(
            inject_base_url("<html><head></head></html>", "https://example.com/&\""),
            "<html><head><base href=\"https://example.com/&amp;&quot;\"></head></html>"
        );
    }
}
