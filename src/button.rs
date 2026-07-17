use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    ButtonProps, Dimension, Rect, StyleContext, TreeContext, matched_style, resolve_font_props,
    style_align_self, style_dimension, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_margin, style_padding_with_default,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, xaml::ButtonElement};

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__winui_Button"];

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

    let button = ButtonElement::new(props.title.get()).expect("failed to create WinUI Button");
    element.provide_handle(button.erased());

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [button, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(button.erased(), Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(button.erased(), Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [button, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&button.erased(), Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state((0.0f32, 0.0f32));
    button
        .set_measure_callback(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI Button measurement");

    scoped_effect!(
        element,
        [button, props.title] || {
            let _ = button.set_title(title.get());
        }
    );

    scoped_effect!(
        element,
        [button, props.disabled] || {
            let _ = button.set_enabled(!disabled.get());
        }
    );

    scoped_effect!(
        element,
        [
            button,
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
            let _ = button.set_font(font);
        }
    );

    scoped_effect!(
        element,
        [button, props.on_click] || {
            let _ = button.set_on_click(on_click.get());
        }
    );

    scoped_effect!(
        element,
        [
            button,
            window_context.scale_factor,
            style_props,
            props.container.padding()
        ] || {
            let padding = style_padding_with_default(
                style_props.get().as_ref(),
                padding.get(),
                Dimension::Auto,
            );
            let _ = button.set_padding(logical_padding(padding, scale_factor.get()));
        }
    );

    scoped_effect!(
        element,
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
        [tree_context, parent_context.parent_node, button] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = button.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );
}

fn logical_padding(padding: Rect<Dimension>, scale_factor: f64) -> Option<Rect<f64>> {
    if [padding.top, padding.bottom, padding.left, padding.right]
        .into_iter()
        .all(|dimension| dimension.is_auto())
    {
        return None;
    }

    let logical = |dimension| match dimension {
        Dimension::Auto => 0.0,
        Dimension::Length(value) => value.to_logical::<f64>(scale_factor).0,
    };
    Some(Rect {
        top: logical(padding.top),
        bottom: logical(padding.bottom),
        left: logical(padding.left),
        right: logical(padding.right),
    })
}

#[cfg(test)]
mod tests {
    use super::logical_padding;
    use nestix_native_core::{Dimension, Rect};

    #[test]
    fn all_auto_padding_preserves_the_native_button_default() {
        assert_eq!(
            logical_padding(
                Rect {
                    top: Dimension::Auto,
                    bottom: Dimension::Auto,
                    left: Dimension::Auto,
                    right: Dimension::Auto,
                },
                1.0,
            ),
            None
        );
    }

    #[test]
    fn explicit_padding_maps_remaining_auto_sides_to_zero() {
        assert_eq!(
            logical_padding(
                Rect {
                    top: Dimension::Auto,
                    bottom: Dimension::Auto,
                    left: Dimension::from(12),
                    right: Dimension::Auto,
                },
                1.0,
            ),
            Some(Rect {
                top: 0.0,
                bottom: 0.0,
                left: 12.0,
                right: 0.0,
            })
        );
    }
}
