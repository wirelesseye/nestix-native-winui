use std::{cell::OnceCell, rc::Rc};

use nestix::{Element, closure, component, components::ContextProvider, layout};
use nestix_native_core::{RootProps, StyleScope};

use crate::{contexts::AppContext, xaml_app::XamlApp};

const DEFAULT_FONT_SIZE: f64 = 14.0;

thread_local! {
    static APP: OnceCell<Rc<XamlApp>> = const { OnceCell::new() };
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__winui_Root"];

    let app =
        APP.with(|slot| {
            slot.get_or_init(|| {
                Rc::new(XamlApp::initialize().expect(
                    "failed to initialize WinUI platform; WinUI requires a Windows STA thread",
                ))
            })
            .clone()
        });

    element.after_mount(closure!(
        [app] || {
            app.run();
        }
    ));

    element.on_unmount(closure!(
        [app] || {
            app.quit();
        }
    ));

    layout! {
        ContextProvider<AppContext>(AppContext { app }) {
            StyleScope(
                .class = props.class.clone(),
                .default_classes = DEFAULT_CLASSES,
                .initial_font_size = DEFAULT_FONT_SIZE,
            ) {
                $(props.children.clone())
            }
        }
    }
}
