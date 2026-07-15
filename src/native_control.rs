use nestix::{Computed, Element, callback, closure, create_state, scoped_effect};
use nestix_native_core::{
    Dimension, ResolvedStyle, TreeContext, ViewProps, style_align_self, style_dimension,
    style_flex_basis, style_flex_grow, style_flex_shrink, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, xaml::XamlElement};

pub(crate) fn mount(
    element: &Element,
    control: XamlElement,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    element.provide_handle(control.clone());
    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [control, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(control.clone(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(control.clone(), Some(node_id));
            }
        }
    ));
    element.on_unmount(closure!(
        [control, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&control, Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state((0.0f32, 0.0f32));
    control
        .set_measure_callback(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI control measurement");

    scoped_effect!(
        element,
        [
            tree_context,
            style_props,
            props.flex_grow,
            props.flex_basis,
            props.flex_shrink,
            window_context.scale_factor
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style.as_ref(), flex_shrink.get()),
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
            intrinsic_size,
            props.width,
            props.height
        ] || {
            let scale = scale_factor.get();
            let style = style_props.get();
            let measured = intrinsic_size.get();
            let width =
                match style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width) {
                    Dimension::Auto => measured.0,
                    Dimension::Length(value) => value.to_logical::<f32>(scale).0,
                };
            let height = match style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| {
                s.height
            }) {
                Dimension::Auto => measured.1,
                Dimension::Length(value) => value.to_logical::<f32>(scale).0,
            };
            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width: taffy::Dimension::from_length(width),
                    height: taffy::Dimension::from_length(height),
                },
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
            props.left,
            props.top
        ] || {
            let style = style_props.get();
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor.get()),
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
            props.margin()
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        element,
        [tree_context, style_props, props.align_self] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, control] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = control.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );
    node_id
}
