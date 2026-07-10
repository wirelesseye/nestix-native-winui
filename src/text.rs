use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    Dimension, StyleContext, TextProps, TreeContext, matched_style, style_align_self,
    style_dimension, style_margin, utils::margin_to_taffy,
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__winui_Text"];

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

    let text_block =
        XamlElement::text_block(props.text.get()).expect("failed to create WinUI TextBlock");
    element.provide_handle(text_block.clone());

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [text_block, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(text_block.clone(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(text_block.clone(), Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [text_block, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&text_block, Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state((0.0f32, 0.0f32));
    text_block
        .set_measure_callback(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI TextBlock measurement");

    scoped_effect!(
        element,
        [text_block, props.text] || {
            let _ = text_block.set_text(text.get());
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
        [tree_context, parent_context.parent_node, text_block] || {
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
