use std::rc::Rc;

use nestix::{
    Element, Layout, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension as NativeDimension, StyleContext, StyleScope, TabViewItemProps, TabViewProps,
    TreeContext, matched_style, style_align_self, style_dimension, style_grow, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Dimension, Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::ParentContext,
    xaml::{TabViewElement, TabViewItemElement, XamlElement},
};

#[derive(Clone)]
struct TabViewContext {
    current_selected: State<Option<String>>,
    content_size: State<(f32, f32)>,
}

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabView", "__winui_TabView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );

    let tab_view = TabViewElement::new().expect("failed to create WinUI SelectorBar tab view");
    element.provide_handle(tab_view.erased());
    let node_id = tree_context.create_node(true);

    let tab_context = TabViewContext {
        current_selected: create_state(None),
        content_size: create_state((0.0, 0.0)),
    };
    tab_view
        .set_selected(callback!([tab_context.current_selected] |id: String| {
            current_selected.set(Some(id));
        }))
        .expect("failed to register SelectorBar selection handler");
    tab_view
        .set_content_resized(
            callback!([tab_context.content_size] |width: f32, height: f32| {
                content_size.set((width, height));
            }),
        )
        .expect("failed to register tab content resize handler");

    element.on_place(closure!(
        [tab_view, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(tab_view.erased(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(tab_view.erased(), Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [tab_view, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&tab_view, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.grow] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_grow(style_props.as_ref(), grow.get()),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                NativeDimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                NativeDimension::Auto,
                |style| style.height,
            );
            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
                    },
                    ..prev
                });
            }
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left = style_dimension(
                style_props.as_ref(),
                left.get(),
                NativeDimension::Auto,
                |style| style.left,
            );
            let top = style_dimension(
                style_props.as_ref(),
                top.get(),
                NativeDimension::Auto,
                |style| style.top,
            );
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, tab_view] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = tab_view.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TabViewContext>(tab_context) {
                ContextProvider<ParentContext>(
                    ParentContext {
                        add_child: Some(callback!([tab_view] |child: XamlElement, _: Option<taffy::NodeId>| {
                            let _ = tab_view.append_child(child);
                        })),
                        insert_child: Some(callback!([tab_view] |child: XamlElement, _: Option<taffy::NodeId>, index: usize| {
                            let _ = tab_view.insert_child(child, index);
                        })),
                        remove_child: Some(callback!([tab_view] |child: &XamlElement, _: Option<taffy::NodeId>| {
                            let _ = tab_view.remove_child(child);
                        })),
                        parent_node: Some(node_id),
                    },
                ) {
                    $(props.children.clone())
                }
            }
        }
    }
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabViewItem", "__winui_TabViewItem"];

    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_context = element.context::<TabViewContext>().unwrap();
    let item = TabViewItemElement::new(props.id.get(), props.title.get())
        .expect("failed to create WinUI SelectorBarItem");
    element.provide_handle(item.erased());

    element.on_place(closure!(
        [item, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(item.erased(), None, index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(item.erased(), None);
            }
        }
    ));

    element.on_unmount(closure!(
        [item, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&item, None);
            }
        }
    ));

    scoped_effect!(
        element,
        [item, props.id] || {
            let _ = item.set_id(id.get());
        }
    );
    scoped_effect!(
        element,
        [item, props.title] || {
            let _ = item.set_title(title.get());
        }
    );
    scoped_effect!(
        element,
        [item, tab_context.current_selected, props.id] || {
            let _ = item.set_visible(current_selected.get() == Some(id.get()));
        }
    );

    let subtree_context = Rc::new(TreeContext::new());
    scoped_effect!(
        element,
        [subtree_context, tab_context.content_size] || {
            let (width, height) = content_size.get();
            if let Some(root_node) = subtree_context.root_node() {
                subtree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: Dimension::from_length(width),
                        height: Dimension::from_length(height),
                    },
                    ..prev
                });
                subtree_context.refresh();
            }
        }
    );

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<ParentContext>(
                    ParentContext {
                        add_child: Some(callback!([item, tab_context.content_size] |child: XamlElement, child_node: Option<taffy::NodeId>| {
                            let _ = item.append_child(child);
                            subtree_context.set_root_node(child_node);
                            if let Some(child_node) = child_node {
                                let (width, height) = content_size.get();
                                subtree_context.update_style(child_node, |prev| Style {
                                    size: Size {
                                        width: Dimension::from_length(width),
                                        height: Dimension::from_length(height),
                                    },
                                    ..prev
                                });
                                subtree_context.refresh();
                            }
                        })),
                        insert_child: None,
                        remove_child: None,
                        parent_node: None,
                    },
                ) {
                    $(props.children.clone().map(|element| Layout::from(element.clone())))
                }
            }
        }
    }
}
