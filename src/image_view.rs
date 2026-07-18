use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    Dimension, ImageViewProps, StyleContext, TreeContext, matched_style, style_align_self,
    style_dimension, style_flex_basis, style_flex_grow, style_flex_shrink, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{
    Size, Style,
    prelude::{FromLength, FromPercent, TaffyAuto},
};

use crate::{WindowContext, contexts::ParentContext, xaml::ImageElement};

#[component]
pub fn ImageView(props: &ImageViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__ImageView", "__winui_ImageView"];
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

    let image = ImageElement::new().expect("failed to create WinUI Image");
    element.provide_handle(image.erased());
    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [image, parent_context] | placement | {
            parent_context.place_child(image.erased(), Some(node_id), placement);
        }
    ));
    element.on_unmount(closure!(
        [image, parent_context] || {
            if let Some(remove) = &parent_context.remove_child {
                remove(&image, Some(node_id));
            }
        }
    ));

    let intrinsic_size = create_state((0.0f32, 0.0f32));
    image
        .set_intrinsic_size_changed(callback!([intrinsic_size] |width: f32, height: f32| {
            intrinsic_size.set((width, height));
        }))
        .expect("failed to register WinUI Image measurement");

    scoped_effect!(
        element,
        [image, props.source] || {
            let _ = image.set_source(source.get());
        }
    );
    scoped_effect!(
        element,
        [image, props.content_fit] || {
            let _ = image.set_content_fit(content_fit.get());
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
            parent_context.parent_node,
            tree_context,
            style_props,
            intrinsic_size,
            props.view.width,
            props.view.height
        ] || {
            let sf = scale_factor.get();
            let style = style_props.get();
            let intrinsic = intrinsic_size.get();
            let width = style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width);
            let height =
                style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| s.height);
            let wa = width.is_auto();
            let ha = height.is_auto();
            let ratio = if intrinsic.1 > 0.0 {
                intrinsic.0 / intrinsic.1
            } else {
                1.0
            };
            let (width, height) = match (width, height) {
                (Dimension::Auto, Dimension::Auto) => intrinsic,
                (Dimension::Length(w), Dimension::Auto) => {
                    let w = w.to_logical::<f32>(sf).0;
                    (w, w / ratio)
                }
                (Dimension::Auto, Dimension::Length(h)) => {
                    let h = h.to_logical::<f32>(sf).0;
                    (h * ratio, h)
                }
                (Dimension::Length(w), Dimension::Length(h)) => {
                    (w.to_logical::<f32>(sf).0, h.to_logical::<f32>(sf).0)
                }
            };
            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    max_size: Size {
                        width: if wa {
                            taffy::Dimension::from_percent(1.0)
                        } else {
                            taffy::Dimension::AUTO
                        },
                        height: if ha {
                            taffy::Dimension::from_percent(1.0)
                        } else {
                            taffy::Dimension::AUTO
                        },
                    },
                    item_is_replaced: true,
                    aspect_ratio: Some(ratio),
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
            let sf = scale_factor.get();
            let style = style_props.get();
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, sf),
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
        [tree_context, style_props, props.view.align_self] || {
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
        [tree_context, parent_context.parent_node, image] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let _ = image.set_layout(
                    layout.location.x.into(),
                    layout.location.y.into(),
                    layout.size.width.into(),
                    layout.size.height.into(),
                );
            }
        }
    );
}
