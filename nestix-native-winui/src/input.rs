use std::rc::Rc;

use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, InputProps, StyleContext, TreeContext, WithAuto, matched_style,
    resolved_view_style, style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_length_with_auto, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{
    Size, Style,
    prelude::{FromLength, TaffyAuto},
};

use crate::{WindowContext, contexts::ParentContext, xaml::TextBoxElement};

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    require_visual_mount!(element, Input);
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__winui_Input"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let matched_style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_style = resolved_view_style(matched_style_props, &props.view);
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

    let text_box = TextBoxElement::new(props.value.get()).expect("failed to create WinUI TextBox");
    element.provide_handle(text_box.erased());

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [text_box, parent_context] | placement | {
            parent_context.place_child(text_box.erased(), Some(node_id), placement);
        }
    ));

    element.on_unmount(closure!(
        [text_box, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&text_box, Some(node_id));
            }
        }
    ));

    let (intrinsic_size, set_intrinsic_size) = create_state((0.0f32, 0.0f32));
    text_box
        .set_measure_callback(callback!([set_intrinsic_size] |width: f32, height: f32| {
            set_intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI TextBox measurement");

    scoped_effect!(
        [text_box, props.value] || {
            let _ = text_box.set_text(value.get());
        }
    );

    scoped_effect!(
        [text_box, props.on_text_change] || {
            let _ = text_box.set_on_text_changed(on_text_change.get().map(|on_text_change| {
                callback!([on_text_change] |text: String| {
                    on_text_change(&text);
                })
            }));
        }
    );

    scoped_effect!(
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
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
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let measured = intrinsic_size.get();
            let width = style_length_with_auto(
                style_props.as_ref(),
                width.get(),
                WithAuto::Auto,
                |style| style.width,
            );
            let height = style_length_with_auto(
                style_props.as_ref(),
                height.get(),
                WithAuto::Auto,
                |style| style.height,
            );
            let (width, min_width) = input_dimension(width, measured.0, scale_factor);
            let (height, min_height) = input_dimension(height, measured.1, scale_factor);

            tree_context.update_style(node_id, |prev| Style {
                size: Size { width, height },
                min_size: Size {
                    width: min_width,
                    height: min_height,
                },
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
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_length_with_auto(style_props.as_ref(), left.get(), WithAuto::Auto, |style| {
                    style.left
                });
            let top =
                style_length_with_auto(style_props.as_ref(), top.get(), WithAuto::Auto, |style| {
                    style.top
                });
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
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
        [tree_context, parent_context.parent_node, text_box] || {
            tree_context.layout_revision().get();
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = text_box.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );
}

fn input_dimension(
    value: WithAuto<nestix_native_core::Length>,
    measured: f32,
    scale_factor: f64,
) -> (taffy::Dimension, taffy::Dimension) {
    match value {
        WithAuto::Auto => (
            taffy::Dimension::AUTO,
            taffy::Dimension::from_length(measured),
        ),
        WithAuto::Value(value) => (
            taffy::Dimension::from_length(value.to_logical::<f32>(scale_factor).0),
            taffy::Dimension::AUTO,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::input_dimension;
    use nestix_native_core::WithAuto;
    use taffy::{
        AlignItems, AvailableSpace, FlexDirection, Size, Style, TaffyTree, prelude::FromLength,
    };

    #[test]
    fn auto_input_width_stretches_past_its_intrinsic_width() {
        let (width, min_width) = input_dimension(WithAuto::Auto, 300.0, 1.0);
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let input = tree
            .new_leaf(Style {
                size: Size {
                    width,
                    height: taffy::Dimension::from_length(32.0),
                },
                min_size: Size {
                    width: min_width,
                    height: taffy::Dimension::from_length(32.0),
                },
                ..Default::default()
            })
            .unwrap();
        let parent = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Column,
                    align_items: Some(AlignItems::Stretch),
                    size: Size {
                        width: taffy::Dimension::from_length(600.0),
                        height: taffy::Dimension::from_length(400.0),
                    },
                    ..Default::default()
                },
                &[input],
            )
            .unwrap();

        tree.compute_layout(
            parent,
            Size {
                width: AvailableSpace::Definite(600.0),
                height: AvailableSpace::Definite(400.0),
            },
        )
        .unwrap();

        assert_eq!(tree.layout(input).unwrap().size.width, 600.0);
    }
}
