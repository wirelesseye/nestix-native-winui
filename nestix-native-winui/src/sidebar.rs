use std::rc::Rc;

use nestix::{
    Element, Layout, callback, closure, component, components::ContextProvider, create_state,
    layout, scoped_effect,
};
use nestix_native_core::{SidebarProps, TreeContext};
use taffy::{Dimension, Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::ParentContext,
    xaml::{SidebarElement, XamlElement},
};

#[component]
pub fn Sidebar(props: &SidebarProps, element: &Element) -> Element {
    require_visual_mount!(element, Sidebar, output);

    let window = element
        .context::<WindowContext>()
        .expect("Sidebar must be mounted beneath a WinUI Window");
    let owner = Rc::new(());
    assert!(
        window.sidebar_owner.borrow().is_none(),
        "a WinUI Window can only contain one mounted Sidebar"
    );
    window.sidebar_owner.replace(Some(owner.clone()));

    let sidebar = SidebarElement::new(
        props.width.get(),
        props.min_width.get(),
        props.resizable.get(),
        props.open.get(),
        props.on_open_change.get(),
    )
    .expect("failed to create WinUI NavigationView sidebar");
    window
        .window
        .set_sidebar(Some(sidebar.clone()))
        .expect("failed to attach WinUI NavigationView sidebar");

    let tree_context = Rc::new(TreeContext::new());
    let (content_size, set_content_size) = create_state((0.0, 0.0));
    sidebar
        .set_content_resized(callback!([set_content_size] |width: f32, height: f32| {
            set_content_size.set((width, height));
        }))
        .expect("failed to watch WinUI sidebar content size");

    element.on_unmount(closure!(
        [window, owner] || {
            let owns_sidebar = window
                .sidebar_owner
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &owner));
            if owns_sidebar {
                window.sidebar_owner.borrow_mut().take();
                let _ = window.window.set_sidebar(None);
            }
        }
    ));

    scoped_effect!(
        [sidebar, props.open] || {
            let _ = sidebar.set_open(open.get());
        }
    );
    scoped_effect!(
        [sidebar, props.on_open_change] || {
            let _ = sidebar.set_on_open_change(on_open_change.get());
        }
    );
    scoped_effect!(
        [sidebar, props.width, props.min_width, props.resizable] || {
            let _ = sidebar.set_sizing(width.get(), min_width.get(), resizable.get());
        }
    );
    scoped_effect!(
        [tree_context, content_size] || {
            let (width, height) = content_size.get();
            if let Some(root_node) = tree_context.root_node() {
                tree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: Dimension::from_length(width),
                        height: Dimension::from_length(height),
                    },
                    ..prev
                });
                tree_context.refresh();
            }
        }
    );

    layout! {
        ContextProvider<TreeContext>(tree_context.clone()) {
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: Some(callback!([sidebar, tree_context, content_size] |child: XamlElement,
                    child_node: Option<taffy::NodeId> | {
                        let _ = sidebar.append_child(child);
                        tree_context.set_root_node(child_node);
                        if let Some(child_node) = child_node {
                            let (width, height) = content_size.get();
                            tree_context.update_style(child_node, |prev| Style {
                                size: Size {
                                    width: Dimension::from_length(width),
                                    height: Dimension::from_length(height),
                                },
                                ..prev
                            });
                            tree_context.refresh();
                        }
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([sidebar] |child: &XamlElement,
                    _: Option<taffy::NodeId> | {
                        let _ = sidebar.remove_child(child);
                    })),
                    parent_node: None
                },
            ) {
                $(props.children.clone().map(|child| Layout::from(child.clone())))
            }
        }
    }
}
