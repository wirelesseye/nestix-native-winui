use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{FlexViewProps, StyleScope};

use crate::{contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__winui_FlexView"];

    let parent_context = element.context::<ParentContext>().unwrap();
    let panel = XamlElement::stack_panel().expect("failed to create WinUI StackPanel");
    element.provide_handle(panel.clone());

    element.on_place(closure!(
        [panel, parent_context] | _ | {
            (parent_context.add_child)(panel.clone());
        }
    ));

    element.on_unmount(closure!(
        [panel, parent_context] || {
            (parent_context.remove_child)(&panel);
        }
    ));

    scoped_effect!(
        element,
        [panel, props.flex_direction] || {
            let _ = panel.set_flex_direction(flex_direction.get());
        }
    );

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: callback!([panel] |child: XamlElement| {
                        let _ = panel.append_child(child);
                    }),
                    remove_child: callback!([panel] |child: &XamlElement| {
                        let _ = panel.remove_child(child);
                    }),
                },
            ) {
                $(props.children.clone())
            }
        }
    }
}
