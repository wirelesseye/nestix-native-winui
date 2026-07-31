use std::{cell::RefCell, rc::Rc};

use nestix::{
    Element, Layout, callback, closure, component, components::ContextProvider, computed,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, AnimationRuntime, Length, StyleContext, StyleScope, TreeContext, WindowProps,
    WithAuto as NativeLengthWithAuto,
    dpi::{LogicalSize, PhysicalSize},
    matched_style, style_length_with_auto,
};
use taffy::{Dimension, Size, Style, prelude::FromLength};

use crate::{
    contexts::{AppContext, ParentContext},
    xaml::{WindowElement, XamlElement},
};

#[derive(Clone)]
pub struct WindowContext {
    pub scale_factor: nestix::Readonly<f64>,
    pub animation: Rc<AnimationRuntime>,
    pub(crate) window: WindowElement,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__winui_Window"];

    let app_context = element.context::<AppContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let (scale_factor, set_scale_factor) = create_state(1.0);
    let tree_context = Rc::new(TreeContext::new());
    let animation = Rc::new(AnimationRuntime::new());
    let content = Rc::new(RefCell::new(None::<(XamlElement, Option<taffy::NodeId>)>));

    let window = WindowElement::new(
        props.title.get(),
        props.desktop.titlebar_mode.get(),
        animation.clone(),
        tree_context.clone(),
    )
    .expect("failed to create WinUI window");
    let window_context = Rc::new(WindowContext {
        scale_factor: scale_factor.clone().into_readonly(),
        animation: animation.clone(),
        window: window.clone(),
    });
    let window_registration = app_context.app.register_window(window.erased());
    window
        .set_scale_factor_changed(Some(callback!([set_scale_factor] |value: f64| {
            set_scale_factor.set(value);
        })))
        .expect("failed to watch WinUI window scale factor");
    element.provide_handle(window.erased());

    scoped_effect!(
        [window, props.title] || {
            let _ = window.set_title(title.get());
        }
    );

    scoped_effect!(
        [window, props.desktop.titlebar_mode] || {
            let _ = window.set_titlebar_mode(titlebar_mode.get());
        }
    );

    scoped_effect!(
        [window, props.visible] || {
            let _ = window.set_visible(visible.get());
        }
    );

    scoped_effect!(
        [window, props.desktop.resizable] || {
            let _ = window.set_resizable(resizable.get());
        }
    );

    window
        .set_resized(Some(callback!([
            tree_context,
            content,
            scale_factor,
            props.on_resize
        ] |size: nestix_native_core::dpi::Size| {
            let logical_size: LogicalSize<f32> = size.to_logical(scale_factor.get());
            sync_window_content(
                &tree_context,
                &content,
                logical_size.width as f64,
                logical_size.height as f64,
            );
            if let Some(on_resize) = on_resize.get() {
                on_resize(size);
            }
        })))
        .expect("failed to watch WinUI window size");

    window
        .set_close_requested(Some(callback!(
            [props.desktop.on_close_requested] || {
                if let Some(on_close_requested) = on_close_requested.get() {
                    on_close_requested();
                }
            }
        )))
        .expect("failed to watch WinUI window close requests");

    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_size = computed!(
        [style_props, props.desktop.width, props.desktop.height] || {
            let mut style = style_props.get().unwrap_or_default();
            style.width = Some(style_length_with_auto(
                Some(&style),
                width.get().into(),
                NativeLengthWithAuto::from(800),
                |style| style.width,
            ));
            style.height = Some(style_length_with_auto(
                Some(&style),
                height.get().into(),
                NativeLengthWithAuto::from(600),
                |style| style.height,
            ));
            Some(style)
        }
    );
    let animated_size = Rc::new(AnimatedStyle::new(animation, target_size.get()));
    let presented_size = animated_size.value();
    scoped_effect!(
        [animated_size, target_size, scale_factor] || {
            animated_size.set_target(target_size.get(), scale_factor.get());
        }
    );
    scoped_effect!(
        [window, tree_context, content, scale_factor, presented_size] || {
            let style = presented_size.get().unwrap_or_default();
            let logical_size = LogicalSize::new(
                logical_length(style.width, 800.0, scale_factor.get()),
                logical_length(style.height, 600.0, scale_factor.get()),
            );
            let physical_size: PhysicalSize<i32> = logical_size.to_physical(scale_factor.get());
            let _ = window.set_size(physical_size.width, physical_size.height);
            sync_window_content(
                &tree_context,
                &content,
                logical_size.width,
                logical_size.height,
            );
        }
    );

    element.on_unmount(closure!(
        [window, window_registration] || {
            let _ = window.set_close_requested(None);
            let _ = window.close();
            window_registration.unregister();
        }
    ));

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                StyleScope(
                    .class = props.class.clone(),
                    .default_classes = DEFAULT_CLASSES,
                    .effective_style = target_size,
                ) {
                    ContextProvider<nestix_native_core::NativeVisualMount>(
                        nestix_native_core::NativeVisualMount::allowed(crate::WINUI_BACKEND_ID),
                    ) {
                        ContextProvider<ParentContext>(
                            ParentContext {
                                add_child: Some(callback!([window, tree_context, content, presented_size, scale_factor] |child: XamlElement,
                                child_node: Option<taffy::NodeId> | {
                                    let style = presented_size.get().unwrap_or_default();
                                    let width =
                                        logical_length(style.width, 800.0, scale_factor.get());
                                    let height =
                                        logical_length(style.height, 600.0, scale_factor.get());
                                    let _ = window.append_child(child.clone());
                                    content.replace(Some((child, child_node)));
                                    tree_context.set_root_node(child_node);
                                    sync_window_content(&tree_context, &content, width, height);
                                })),
                                insert_child: None,
                                remove_child: Some(callback!([window, tree_context, content] |child: &XamlElement,
                                _: Option<taffy::NodeId> | {
                                    let _ = window.remove_child(child);
                                    if content
                                        .borrow()
                                        .as_ref()
                                        .is_some_and(|(current, _)| current == child)
                                    {
                                        content.borrow_mut().take();
                                    }
                                    tree_context.set_root_node(None);
                                })),
                                parent_node: None
                            },
                        ) {
                            $(props.children.clone().map(|element| Layout::from(element.clone())))
                        }
                    }
                }
            }
        }
    }
}

fn sync_window_content(
    tree_context: &TreeContext,
    content: &RefCell<Option<(XamlElement, Option<taffy::NodeId>)>>,
    width: f64,
    height: f64,
) {
    let current = content.borrow().clone();
    let Some((content, root_node)) = current else {
        return;
    };
    // The window content must remain auto-sized and stretched by WinUI. Giving
    // it an explicit width/height prevents it from tracking the client area,
    // which in turn means its SizeChanged handler never sees window resizes.
    // Only Taffy's root receives the concrete client size.
    let _ = content;
    if let Some(root_node) = root_node {
        tree_context.update_style(root_node, |prev| Style {
            size: Size {
                width: Dimension::from_length(width as f32),
                height: Dimension::from_length(height as f32),
            },
            ..prev
        });
        tree_context.refresh();
    }
}

fn logical_length(
    value: Option<NativeLengthWithAuto<Length>>,
    fallback: f64,
    scale_factor: f64,
) -> f64 {
    match value {
        Some(NativeLengthWithAuto::Value(value)) => value.to_logical::<f64>(scale_factor).0,
        Some(NativeLengthWithAuto::Auto) | None => fallback,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use nestix_native_core::TreeContext;

    use super::sync_window_content;
    use crate::xaml::CanvasElement;

    #[test]
    fn window_content_size_updates_layout_tree_without_fixing_native_root_size() {
        let tree = TreeContext::new();
        let root = tree.create_node(false);
        tree.set_root_node(Some(root));
        let canvas = CanvasElement::new().unwrap();
        let content = RefCell::new(Some((canvas.erased(), Some(root))));

        sync_window_content(&tree, &content, 900.0, 620.0);

        assert_eq!(canvas.cached_layout(), None);
        let layout = tree.layout(root).unwrap();
        assert_eq!((layout.size.width, layout.size.height), (900.0, 620.0));
    }

    #[test]
    fn window_content_size_is_safe_before_content_mounts() {
        let tree = TreeContext::new();
        let content = RefCell::new(None);
        sync_window_content(&tree, &content, 900.0, 620.0);
    }
}
