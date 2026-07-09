use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::TextProps;

use crate::{contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    let parent_context = element.context::<ParentContext>().unwrap();
    let text_block =
        XamlElement::text_block(props.text.get()).expect("failed to create WinUI TextBlock");
    element.provide_handle(text_block.clone());

    element.on_place(closure!(
        [text_block, parent_context] | _ | {
            (parent_context.add_child)(text_block.clone());
        }
    ));

    element.on_unmount(closure!(
        [text_block, parent_context] || {
            (parent_context.remove_child)(&text_block);
        }
    ));

    scoped_effect!(
        element,
        [text_block, props.text] || {
            let _ = text_block.set_text(text.get());
        }
    );
}
