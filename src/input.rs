use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    Dimension, InputProps, StyleContext, TreeContext, matched_style, style_align_self,
    style_dimension, style_grow, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, xaml::TextBoxElement};

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__winui_Input"];

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

    let text_box = TextBoxElement::new(props.value.get()).expect("failed to create WinUI TextBox");
    element.provide_handle(text_box.erased());

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [text_box, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(text_box.erased(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(text_box.erased(), Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [text_box, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&text_box, Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state((0.0f32, 0.0f32));
    text_box
        .set_measure_callback(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI TextBox measurement");

    scoped_effect!(
        element,
        [text_box, props.value] || {
            let _ = text_box.set_text(value.get());
        }
    );

    scoped_effect!(
        element,
        [text_box, props.on_text_change] || {
            let _ = text_box.set_on_text_changed(on_text_change.get().map(|on_text_change| {
                callback!([on_text_change] |text: String| {
                    on_text_change(&text);
                })
            }));
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
            style_props,
            intrinsic_size,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let measured = intrinsic_size.get();
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
            let width = match width {
                Dimension::Auto => measured.0,
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor).0,
            };
            let height = match height {
                Dimension::Auto => measured.1,
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor).0,
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
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
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
        [tree_context, parent_context.parent_node, text_box] || {
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
