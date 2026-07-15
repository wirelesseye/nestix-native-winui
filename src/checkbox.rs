use nestix::{Element, callback, component, scoped_effect};
use nestix_native_core::{CheckboxProps, StyleContext, matched_style, resolve_font_props};

use crate::{native_control, xaml::CheckBoxElement};

#[component]
pub fn Checkbox(props: &CheckboxProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Checkbox", "__winui_Checkbox"];
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let control = CheckBoxElement::new(props.title.get()).expect("failed to create WinUI CheckBox");
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
        [control, props.checked] || {
            let _ = control.set_checked(checked.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.on_checked_change] || {
            let _ = control.set_on_checked_change(
                on_checked_change
                    .get()
                    .map(|handler| callback!([handler] |checked: bool| handler(checked))),
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
