use std::rc::Rc;

use nestix::{Computed, Element, callback, closure, create_state, scoped_effect};
use nestix_native_core::{
    AlignItems, AnimatedStyle, Length, ResolvedStyle, TreeContext, ViewProps, WithAuto,
    resolved_view_style, style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_length_with_auto, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{
    NodeId, Size, Style,
    prelude::{FromLength, TaffyAuto},
};

use crate::{WindowContext, contexts::ParentContext, xaml::XamlElement};

fn preserve_intrinsic_size(previous: (f32, f32), measured: (f32, f32)) -> (f32, f32) {
    (
        if measured.0 > 0.0 {
            measured.0
        } else {
            previous.0
        },
        if measured.1 > 0.0 {
            measured.1
        } else {
            previous.1
        },
    )
}

fn leaf_dimension(
    value: WithAuto<Length>,
    measured: f32,
    scale: f64,
    stretch: bool,
) -> taffy::Dimension {
    match value {
        WithAuto::Auto if stretch => taffy::Dimension::AUTO,
        WithAuto::Auto => taffy::Dimension::from_length(measured),
        WithAuto::Value(value) => taffy::Dimension::from_length(value.to_logical::<f32>(scale).0),
    }
}

pub(crate) fn mount(
    element: &Element,
    control: XamlElement,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
) -> NodeId {
    mount_impl(element, control, style_props, props, (0.0, 0.0), false)
}

pub(crate) fn mount_with_intrinsic_size(
    element: &Element,
    control: XamlElement,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    fallback_intrinsic_size: (f32, f32),
) -> NodeId {
    mount_impl(
        element,
        control,
        style_props,
        props,
        fallback_intrinsic_size,
        true,
    )
}

fn mount_impl(
    element: &Element,
    control: XamlElement,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    fallback_intrinsic_size: (f32, f32),
    stretch_auto_size: bool,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let target_style = resolved_view_style(style_props, props);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        target_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, target_style, window_context.scale_factor] || {
            animated_style.set_target(target_style.get(), scale_factor.get());
        }
    );

    element.provide_handle(control.clone());
    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [control, parent_context] | placement | {
            parent_context.place_child(control.clone(), Some(node_id), placement);
        }
    ));
    element.on_unmount(closure!(
        [control, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&control, Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state(fallback_intrinsic_size);
    control
        .set_measure_callback(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size
                .update(|previous| preserve_intrinsic_size(*previous, (width, height)));
        }))
        .expect("failed to register WinUI control measurement");

    scoped_effect!(
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
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            intrinsic_size,
            props.width,
            props.height,
            props.align_self
        ] || {
            let scale = scale_factor.get();
            let style = style_props.get();
            let measured = intrinsic_size.get();
            let stretch = stretch_auto_size
                && style_align_self(style.as_ref(), align_self.get()) == AlignItems::Stretch;
            let width = leaf_dimension(
                style_length_with_auto(style.as_ref(), width.get(), WithAuto::Auto, |style| {
                    style.width
                }),
                measured.0,
                scale,
                stretch,
            );
            let height = leaf_dimension(
                style_length_with_auto(style.as_ref(), height.get(), WithAuto::Auto, |style| {
                    style.height
                }),
                measured.1,
                scale,
                stretch,
            );
            tree_context.update_style(node_id, |prev| Style {
                size: Size { width, height },
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.left,
            props.top
        ] || {
            let style = style_props.get();
            let left =
                style_length_with_auto(style.as_ref(), left.get(), WithAuto::Auto, |s| s.left);
            let top = style_length_with_auto(style.as_ref(), top.get(), WithAuto::Auto, |s| s.top);
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor.get()),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
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
        [tree_context, parent_context.parent_node, control] || {
            tree_context.layout_revision().get();
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

#[cfg(test)]
mod tests {
    use super::{leaf_dimension, preserve_intrinsic_size};
    use nestix_native_core::WithAuto;
    use taffy::prelude::{FromLength, TaffyAuto};

    #[test]
    fn zero_measurement_preserves_a_leaf_fallback_size() {
        assert_eq!(
            preserve_intrinsic_size((300.0, 150.0), (0.0, 0.0)),
            (300.0, 150.0)
        );
        assert_eq!(
            preserve_intrinsic_size((300.0, 150.0), (640.0, 0.0)),
            (640.0, 150.0)
        );
    }

    #[test]
    fn stretched_leaf_keeps_its_auto_dimension() {
        assert_eq!(
            leaf_dimension(WithAuto::Auto, 300.0, 1.0, true),
            taffy::Dimension::AUTO
        );
        assert_eq!(
            leaf_dimension(WithAuto::Auto, 300.0, 1.0, false),
            taffy::Dimension::from_length(300.0)
        );
    }
}
