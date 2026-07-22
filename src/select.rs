use std::cell::Cell;

use nestix::{
    Element, Shared, callback, closure, component, components::ContextProvider, layout,
    scoped_effect,
};
use nestix_native_core::{SelectOptionProps, SelectProps, StyleContext, matched_style};

use crate::{
    native_control,
    xaml::{SelectElement, SelectOptionData},
};

thread_local! { static NEXT_OPTION_ID: Cell<u64> = const { Cell::new(1) }; }

#[derive(Clone)]
struct SelectContext {
    upsert: Shared<dyn Fn(u64, SelectOptionData)>,
    move_to: Shared<dyn Fn(u64, usize)>,
    remove: Shared<dyn Fn(u64)>,
}

#[component]
pub fn Select(props: &SelectProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Select", "__winui_Select"];
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let control = SelectElement::new().expect("failed to create WinUI ComboBox");
    native_control::mount(element, control.erased(), style, &props.view);
    scoped_effect!(
        element,
        [control, props.enabled] || {
            let _ = control.set_enabled(enabled.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.value] || {
            let _ = control.set_value(value.get());
        }
    );
    scoped_effect!(
        element,
        [control, props.on_value_change] || {
            let _ = control.set_on_value_change(
                on_value_change
                    .get()
                    .map(|handler| callback!([handler] |value: String| handler(&value))),
            );
        }
    );
    layout! {
        ContextProvider<SelectContext>(
            SelectContext {
                upsert: callback!(
                    [control] |id: u64, option: SelectOptionData| {
                        let _ = control.upsert_option(id, option);
                    }
                ),
                move_to: callback!(
                    [control] |id: u64, index: usize| {
                        let _ = control.move_option(id, index);
                    }
                ),
                remove: callback!(
                    [control] |id: u64| {
                        let _ = control.remove_option(id);
                    }
                )
            },
        ) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn SelectOption(props: &SelectOptionProps, element: &Element) {
    let context = element
        .context::<SelectContext>()
        .expect("SelectOption must be a child of Select");
    let id = NEXT_OPTION_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1).max(1));
        id
    });
    element.on_place(closure!(
        [context] | placement | {
            if let Some(index) = placement.index {
                (context.move_to)(id, index);
            }
        }
    ));
    element.on_unmount(closure!([context] || (context.remove)(id)));
    scoped_effect!(
        element,
        [context, props.label, props.value, props.enabled] || {
            (context.upsert)(
                id,
                SelectOptionData {
                    label: label.get(),
                    value: value.get(),
                    enabled: enabled.get(),
                },
            );
        }
    );
}
