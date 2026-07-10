use std::rc::Rc;

use nestix::{
    Element, callback, closure, component, components::ContextProvider, create_state, layout,
    scoped_effect,
};
use nestix_native_core::{
    StyleScope, TreeContext, WindowProps,
    dpi::{LogicalSize, PhysicalSize},
};
use taffy::{Dimension, Size, Style, prelude::FromLength};

use crate::{
    contexts::{AppContext, ParentContext},
    xaml::XamlElement,
};

#[derive(Clone)]
pub struct WindowContext {
    pub scale_factor: nestix::Readonly<f64>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__winui_Window"];

    let app_context = element.context::<AppContext>().unwrap();
    let scale_factor = create_state(1.0);
    let window_context = Rc::new(WindowContext {
        scale_factor: scale_factor.clone().into_readonly(),
    });
    let tree_context = Rc::new(TreeContext::new());

    let window = XamlElement::window(props.title.get()).expect("failed to create WinUI window");
    let window_registration = app_context.app.register_window(window.clone());
    window
        .set_scale_factor_changed(Some(callback!([scale_factor] |value: f64| {
            scale_factor.set(value);
        })))
        .expect("failed to watch WinUI window scale factor");
    element.provide_handle(window.clone());

    element.after_mount(closure!(
        [window] || {
            let _ = window.activate();
        }
    ));

    scoped_effect!(
        element,
        [window, props.title] || {
            let _ = window.set_text(title.get());
        }
    );

    window
        .set_resized(Some(callback!([
            tree_context,
            scale_factor,
            props.on_resize
        ] |size: nestix_native_core::dpi::Size| {
            let logical_size: LogicalSize<f32> = size.to_logical(scale_factor.get());
            if let Some(root_node) = tree_context.root_node() {
                tree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: Dimension::from_length(logical_size.width),
                        height: Dimension::from_length(logical_size.height),
                    },
                    ..prev
                });
                tree_context.refresh();
            }
            if let Some(on_resize) = on_resize.get() {
                on_resize(size);
            }
        })))
        .expect("failed to watch WinUI window size");

    scoped_effect!(
        element,
        [
            window,
            tree_context,
            scale_factor,
            props.width,
            props.height
        ] || {
            let logical_size = LogicalSize::new(width.get(), height.get());
            let physical_size: PhysicalSize<i32> = logical_size.to_physical(scale_factor.get());
            let _ = window.set_window_size(physical_size.width, physical_size.height);
            if let Some(root_node) = tree_context.root_node() {
                tree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: Dimension::from_length(width.get() as f32),
                        height: Dimension::from_length(height.get() as f32),
                    },
                    ..prev
                });
                tree_context.refresh();
            }
        }
    );

    element.on_unmount(closure!(
        [window_registration] || {
            window_registration.unregister();
        }
    ));

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                    ContextProvider<ParentContext>(
                        ParentContext {
                            add_child: Some(callback!([window, tree_context, props.width, props.height] |child: XamlElement, child_node: Option<taffy::NodeId>| {
                                let _ = child.set_layout(0.0, 0.0, width.get(), height.get());
                                let _ = window.append_child(child);
                                tree_context.set_root_node(child_node);
                                if let Some(child_node) = child_node {
                                    tree_context.update_style(child_node, |prev| Style {
                                        size: Size {
                                            width: Dimension::from_length(width.get() as f32),
                                            height: Dimension::from_length(height.get() as f32),
                                        },
                                        ..prev
                                    });
                                    tree_context.refresh();
                                }
                            })),
                            insert_child: None,
                            remove_child: Some(callback!([window, tree_context] |child: &XamlElement, _: Option<taffy::NodeId>| {
                                let _ = window.remove_child(child);
                                tree_context.set_root_node(None);
                            })),
                            parent_node: None,
                        },
                    ) {
                        $(props.children.get())
                    }
                }
            }
        }
    }
}
