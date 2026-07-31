use std::rc::Rc;

use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, StyleContext, TextProps, TreeContext, WithAuto, matched_style,
    resolve_font_props, resolved_view_style, style_align_self, style_flex_basis, style_flex_grow,
    style_flex_shrink, style_length_with_auto, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, xaml::TextBlockElement};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    require_visual_mount!(element, Text);
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__winui_Text"];

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

    let text_block =
        TextBlockElement::new(props.text.get()).expect("failed to create WinUI TextBlock");
    element.provide_handle(text_block.erased());

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [text_block, parent_context] | placement | {
            parent_context.place_child(text_block.erased(), Some(node_id), placement);
        }
    ));

    element.on_unmount(closure!(
        [text_block, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&text_block, Some(node_id));
            }
        }
    ));

    let (intrinsic_size, set_intrinsic_size) = create_state((0.0f32, 0.0f32));
    text_block
        .set_measure_callback(callback!([set_intrinsic_size] |width: f32, height: f32| {
            set_intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI TextBlock measurement");

    scoped_effect!(
        [text_block, props.text] || {
            let _ = text_block.set_text(text.get());
        }
    );

    scoped_effect!(
        [
            text_block,
            style_props,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color
        ] || {
            let font = resolve_font_props(
                style_props.get().as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let _ = text_block.set_font(font);
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
            let width = match width {
                WithAuto::Auto => measured.0,
                WithAuto::Value(length) => length.to_logical::<f32>(scale_factor).0,
            };
            let height = match height {
                WithAuto::Auto => measured.1,
                WithAuto::Value(length) => length.to_logical::<f32>(scale_factor).0,
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
        [tree_context, parent_context.parent_node, text_block] || {
            tree_context.layout_revision().get();
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = text_block.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );
}
