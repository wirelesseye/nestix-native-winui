use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::ButtonProps;

use crate::{contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    let parent_context = element.context::<ParentContext>().unwrap();
    let button = XamlElement::button(props.title.get()).expect("failed to create WinUI Button");
    element.provide_handle(button.clone());

    element.on_place(closure!(
        [button, parent_context] | _ | {
            (parent_context.add_child)(button.clone());
        }
    ));

    element.on_unmount(closure!(
        [button, parent_context] || {
            (parent_context.remove_child)(&button);
        }
    ));

    scoped_effect!(
        element,
        [button, props.title] || {
            let _ = button.set_text(title.get());
        }
    );

    scoped_effect!(
        element,
        [button, props.on_click] || {
            let _ = button.set_button_click(on_click.get());
        }
    );
}
