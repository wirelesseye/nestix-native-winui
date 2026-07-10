use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension, FlexViewProps, StyleContext, StyleScope, TreeContext, matched_style,
    style_align_items, style_align_self, style_dimension, style_flex_direction, style_flex_wrap,
    style_grow, style_justify_content, style_margin, style_padding,
    utils::{margin_to_taffy, padding_to_taffy},
};
use taffy::{Size, Style};

use crate::{WindowContext, contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__winui_FlexView"];

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

    let canvas = XamlElement::canvas().expect("failed to create WinUI Canvas");
    element.provide_handle(canvas.clone());

    let node_id = tree_context.create_node(false);
    element.on_place(closure!(
        [canvas, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(canvas.clone(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(canvas.clone(), Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [canvas, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&canvas, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [canvas, style_props, props.bg_color] || {
            let style_props = style_props.get();
            let bg_color = bg_color.get().or_else(|| {
                style_props
                    .as_ref()
                    .and_then(|style_props| style_props.bg_color)
            });
            let _ = canvas.set_background_color(bg_color);
        }
    );

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
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
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
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
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
            props.padding()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                padding: padding_to_taffy(
                    style_padding(style_props.as_ref(), padding.get()),
                    scale_factor,
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
        [tree_context, style_props, props.flex_direction] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: style_flex_direction(style_props.as_ref(), flex_direction.get())
                    .to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.align_items] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_items: style_align_items(style_props.as_ref(), align_items.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.justify_content] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                justify_content: style_justify_content(style_props.as_ref(), justify_content.get())
                    .to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.flex_wrap] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: style_flex_wrap(style_props.as_ref(), flex_wrap.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, canvas] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = canvas.set_layout(
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
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: Some(callback!([tree_context, canvas] |child: XamlElement, child_node: Option<taffy::NodeId>| {
                        if canvas.contains_child(&child)
                            && let Some(child_node) = child_node
                        {
                            tree_context.remove_child(node_id, child_node);
                        }
                        let _ = canvas.append_child(child);
                        if let Some(child_node) = child_node {
                            tree_context.add_child(node_id, child_node);
                            tree_context.refresh();
                        }
                    })),
                    insert_child: Some(callback!([tree_context, canvas] |child: XamlElement, child_node: Option<taffy::NodeId>, index: usize| {
                        if canvas.contains_child(&child)
                            && let Some(child_node) = child_node
                        {
                            tree_context.remove_child(node_id, child_node);
                        }
                        let _ = canvas.insert_child(child.clone(), index);
                        if let Some(child_node) = child_node {
                            // A Nestix placement index can include elements for which this
                            // backend produced no native child (for example, an unsupported
                            // control). Use the Canvas order so Taffy receives an index in its
                            // own, filtered child list.
                            let layout_index = canvas.child_index(&child).unwrap();
                            tree_context.insert_child(node_id, child_node, layout_index);
                            tree_context.refresh();
                        }
                    })),
                    remove_child: Some(callback!([tree_context, canvas] |child: &XamlElement, child_node: Option<taffy::NodeId>| {
                        let _ = canvas.remove_child(child);
                        if let Some(child_node) = child_node {
                            tree_context.remove_child(node_id, child_node);
                            tree_context.refresh();
                        }
                    })),
                    parent_node: Some(node_id),
                },
            ) {
                $(props.children.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nestix_native_core::TreeContext;
    use taffy::{
        AlignItems, Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, Rect, Size,
        Style, prelude::FromLength,
    };

    #[test]
    fn tree_context_computes_row_grow_alignment_padding_and_margin() {
        let tree = TreeContext::new();
        let root = tree.create_node(false);
        let fixed = tree.create_node(true);
        let growing = tree.create_node(true);

        tree.update_style(root, |prev| Style {
            size: Size {
                width: Dimension::from_length(200.0),
                height: Dimension::from_length(100.0),
            },
            padding: Rect {
                left: LengthPercentage::length(10.0),
                right: LengthPercentage::length(10.0),
                top: LengthPercentage::length(10.0),
                bottom: LengthPercentage::length(10.0),
            },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            ..prev
        });
        tree.update_style(fixed, |prev| Style {
            size: Size {
                width: Dimension::from_length(20.0),
                height: Dimension::from_length(20.0),
            },
            margin: Rect {
                left: LengthPercentageAuto::length(5.0),
                ..Rect::zero()
            },
            ..prev
        });
        tree.update_style(growing, |prev| Style {
            size: Size {
                width: Dimension::from_length(20.0),
                height: Dimension::from_length(20.0),
            },
            flex_grow: 1.0,
            ..prev
        });
        tree.add_child(root, fixed);
        tree.add_child(root, growing);
        tree.set_root_node(Some(root));
        tree.refresh();

        let fixed_layout = tree.layout(fixed).unwrap();
        let growing_layout = tree.layout(growing).unwrap();
        assert_eq!(
            (fixed_layout.location.x, fixed_layout.location.y),
            (15.0, 40.0)
        );
        assert_eq!(
            (fixed_layout.size.width, fixed_layout.size.height),
            (20.0, 20.0)
        );
        assert_eq!(
            (growing_layout.location.x, growing_layout.location.y),
            (35.0, 40.0)
        );
        assert_eq!(
            (growing_layout.size.width, growing_layout.size.height),
            (155.0, 20.0)
        );
    }
}
