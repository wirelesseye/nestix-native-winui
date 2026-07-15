use nestix::{Element, callback, component, scoped_effect};
use nestix_native_core::{StyleContext, SwitchProps, matched_style};

use crate::{native_control, xaml::SwitchElement};

#[component]
pub fn Switch(props: &SwitchProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Switch", "__winui_Switch"];
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let control = SwitchElement::new().expect("failed to create WinUI ToggleSwitch");
    native_control::mount(element, control.erased(), style, &props.view);
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
}
