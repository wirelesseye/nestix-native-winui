use nestix::{Element, callback, component, scoped_effect};
use nestix_native_core::{RadioButtonProps, StyleContext, matched_style, resolve_font_props};

use crate::{native_control, xaml::RadioButtonElement};

#[component]
pub fn RadioButton(props: &RadioButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__RadioButton", "__winui_RadioButton"];
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let control =
        RadioButtonElement::new(props.title.get()).expect("failed to create WinUI RadioButton");
    native_control::mount(element, control.erased(), style.clone(), &props.view);

    scoped_effect!(
        element,
        [control, props.title] || {
            let _ = control.set_title(title.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.enabled] || {
            let _ = control.set_enabled(enabled.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.group] || {
            let _ = control.set_group(group.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.selected] || {
            let _ = control.set_selected(selected.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.on_select] || {
            let _ = control.set_on_select(
                on_select
                    .get()
                    .map(|handler| callback!([handler] || handler())),
            );
        }
    );
    scoped_effect!(
        element,
        [
            control,
            style,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color
        ] || {
            let font = resolve_font_props(
                style.get().as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let _ = control.set_font(font);
        }
    );
}
