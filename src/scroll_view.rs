use std::rc::Rc;

use nestix::{
    Element, Layout, callback, closure, component, components::ContextProvider, layout,
    scoped_effect,
};
use nestix_native_core::{
    Dimension, ScrollViewProps, StyleContext, StyleScope, TreeContext, matched_style,
    style_align_self, style_dimension, style_flex_grow, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, style_helpers::FromLength};

use crate::{
    WindowContext,
    contexts::ParentContext,
    xaml::{ScrollViewElement, XamlElement},
};

#[component]
pub fn ScrollView(props: &ScrollViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__ScrollView", "__winui_ScrollView"];

    let window = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let scroll = ScrollViewElement::new().expect("failed to create WinUI ScrollView");
    element.provide_handle(scroll.erased());
    let node = tree_context.create_node(false);

    element.on_place(closure!(
        [scroll, parent] | placement | {
            if let Some(index) = placement.index
                && let Some(insert) = &parent.insert_child
            {
                insert(scroll.erased(), Some(node), index);
            } else if let Some(add) = &parent.add_child {
                add(scroll.erased(), Some(node));
            }
        }
    ));

    element.on_unmount(closure!(
        [scroll, parent] || {
            if let Some(remove) = &parent.remove_child {
                remove(&scroll, Some(node));
            }
        }
    ));

    scoped_effect!(
        element,
        [scroll, props.scroll_x, props.scroll_y] || {
            let _ = scroll.set_scroll_enabled(scroll_x.get(), scroll_y.get());
        }
    );

    scoped_effect!(
        element,
        [tree_context, styles, props.view.flex_grow, props.view.align_self] || {
            let style = styles.get();
            tree_context.update_style(node, |prev| Style {
                flex_grow: style_flex_grow(style.as_ref(), flex_grow.get()),
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window.scale_factor,
            tree_context,
            styles,
            props.view.width,
            props.view.height,
            props.view.left,
            props.view.top,
            props.view.margin()
        ] || {
            let scale = scale_factor.get();
            let style = styles.get();
            let width = style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width);
            let height =
                style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| s.height);
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node, |prev| Style {
                flex_direction: taffy::FlexDirection::Column,
                size: Size {
                    width: width.to_taffy(scale),
                    height: height.to_taffy(scale),
                },
                inset: inset_to_taffy(left, top, scale),
                margin: margin_to_taffy(style_margin(style.as_ref(), margin.get()), scale),
                ..prev
            });
            tree_context.refresh();
        }
    );

    let subtree_context = Rc::new(TreeContext::new());
    let subtree_root = subtree_context.create_node(false);
    subtree_context.set_root_node(Some(subtree_root));

    scoped_effect!(
        element,
        [tree_context, subtree_context, parent.parent_node, scroll] || {
            if parent_node.is_some()
                && let Some(value) = tree_context.layout(node)
            {
                let _ = scroll.set_layout(
                    value.location.x.into(),
                    value.location.y.into(),
                    value.size.width.into(),
                    value.size.height.into(),
                );
                subtree_context.update_style(subtree_root, |prev| Style {
                    min_size: Size {
                        width: taffy::Dimension::from_length(value.size.width),
                        height: taffy::Dimension::from_length(value.size.height),
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
                ContextProvider<ParentContext>(ParentContext {
                    add_child: Some(callback!([scroll, subtree_context] |child: XamlElement, child_node: Option<taffy::NodeId>| {
                        let _ = scroll.append_child(child);
                        if let Some(child_node) = child_node {
                            subtree_context.add_child(subtree_root, child_node);
                            subtree_context.refresh();
                        }
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([scroll, subtree_context] |child: &XamlElement, child_node: Option<taffy::NodeId>| {
                        let _ = scroll.remove_child(child);
                        if let Some(child_node) = child_node {
                            subtree_context.remove_child(subtree_root, child_node);
                            subtree_context.refresh();
                        }
                    })),
                    parent_node: Some(subtree_root),
                }) {
                    $(props.children.clone().map(|element| Layout::from(element.clone())))
                }
            }
        }
    }
}
