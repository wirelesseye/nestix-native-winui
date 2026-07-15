use nestix::{Element, callback, component, scoped_effect};
use nestix_native_core::{SliderProps, StyleContext, matched_style};

use crate::{native_control, xaml::SliderElement};

#[component]
pub fn Slider(props: &SliderProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Slider", "__winui_Slider"];
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let control = SliderElement::new().expect("failed to create WinUI Slider");
    native_control::mount(element, control.erased(), style, &props.view);
    scoped_effect!(
        element,
        [control, props.enabled] || {
            let _ = control.set_enabled(enabled.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.minimum, props.maximum, props.value] || {
            let _ = control.set_range(minimum.get(), maximum.get(), value.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.on_value_change] || {
            let _ = control.set_on_value_change(
                on_value_change
                    .get()
                    .map(|handler| callback!([handler] |value: f64| handler(value))),
            );
        }
    );
}
