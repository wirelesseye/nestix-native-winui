use std::{cell::OnceCell, rc::Rc};

use nestix::{Element, closure, component, components::ContextProvider, layout};
use nestix_native_core::{RootProps, StyleScope};

use crate::{contexts::AppContext, xaml_app::XamlApp};

thread_local! {
    static APP: OnceCell<Rc<XamlApp>> = const { OnceCell::new() };
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__winui_Root"];

    let app = APP.with(|slot| {
        slot.get_or_init(|| {
            Rc::new(
                XamlApp::initialize(props.quit_when_all_windows_closed.clone()).expect(
                    "failed to initialize WinUI platform; WinUI requires a Windows STA thread",
                ),
            )
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
            StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                $(props.children.clone())
            }
        }
    }
}
