use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use nestix::{
    Element, PropValue, callback, closure, component, components::ContextProvider, layout,
    scoped_effect,
};
use nestix_native_core::{
    NavigationItemProps, SidebarNavigationProps, StyleContext, StyleScope, matched_style,
    resolved_view_style,
};

use crate::{sidebar::SidebarContext, xaml::NavigationItemElement};

static NEXT_NAVIGATION_ITEM_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct NavigationItemRegistration {
    item: NavigationItemElement,
    value: PropValue<String>,
}

#[derive(Clone)]
struct SidebarNavigationContext {
    sidebar: crate::xaml::SidebarElement,
    items: Rc<RefCell<Vec<NavigationItemRegistration>>>,
    selected_value: PropValue<Option<String>>,
}

impl SidebarNavigationContext {
    fn sync_selection(&self) {
        let value = self.selected_value.get();
        let items = self.items.borrow();
        let selected = value.as_deref().and_then(|value| {
            items
                .iter()
                .find(|registration| registration.value.get() == value)
                .map(|registration| &registration.item)
        });
        let _ = self.sidebar.set_navigation_selected(selected);
    }

    fn assert_unique_values(&self) {
        let items = self.items.borrow();
        for (index, item) in items.iter().enumerate() {
            let value = item.value.get();
            assert!(
                !items[..index]
                    .iter()
                    .any(|previous| previous.value.get() == value),
                "NavigationItem values must be unique within SidebarNavigation: {value:?}"
            );
        }
    }
}

#[component]
pub fn SidebarNavigation(props: &SidebarNavigationProps, element: &Element) -> Element {
    require_visual_mount!(element, SidebarNavigation, output);
    const DEFAULT_CLASSES: [&str; 2] = ["__SidebarNavigation", "__winui_SidebarNavigation"];

    let sidebar_context = element
        .context::<SidebarContext>()
        .expect("SidebarNavigation must be mounted beneath Sidebar");
    let owner = Rc::new(());
    assert!(
        sidebar_context.navigation_owner.borrow().is_none(),
        "a WinUI Sidebar can only contain one mounted SidebarNavigation"
    );
    sidebar_context
        .navigation_owner
        .replace(Some(owner.clone()));

    let matched_styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched_styles, &props.view);
    let context = SidebarNavigationContext {
        sidebar: sidebar_context.sidebar.clone(),
        items: Rc::new(RefCell::new(Vec::new())),
        selected_value: props.value.clone(),
    };

    context
        .sidebar
        .set_navigation_selected_handler(Some(callback!([
            context.items,
            props.on_value_change
        ] |id: String| {
            let value = items
                .borrow()
                .iter()
                .find(|registration| registration.item.id() == id)
                .map(|registration| registration.value.get());
            if let (Some(value), Some(on_value_change)) = (value, on_value_change.get()) {
                on_value_change(&value);
            }
        })))
        .expect("failed to watch WinUI sidebar navigation selection");

    element.on_unmount(closure!(
        [sidebar_context, owner] || {
            let owns_navigation = sidebar_context
                .navigation_owner
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &owner));
            if owns_navigation {
                sidebar_context.navigation_owner.borrow_mut().take();
                let _ = sidebar_context
                    .sidebar
                    .set_navigation_selected_handler(None);
            }
        }
    ));

    scoped_effect!(
        [context, props.value] || {
            let _ = value.get();
            context.sync_selection();
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<SidebarNavigationContext>(context) {
                $(props.children.clone())
            }
        }
    }
}

#[component]
pub fn NavigationItem(props: &NavigationItemProps, element: &Element) {
    require_visual_mount!(element, NavigationItem);
    let context = element
        .context::<SidebarNavigationContext>()
        .expect("NavigationItem must be mounted beneath SidebarNavigation");
    let id = format!(
        "nestixNavigationItem{}",
        NEXT_NAVIGATION_ITEM_ID.fetch_add(1, Ordering::Relaxed)
    );
    let item = NavigationItemElement::new(id, props.label.get(), props.enabled.get())
        .expect("failed to create WinUI NavigationViewItem");
    let registration = NavigationItemRegistration {
        item: item.clone(),
        value: props.value.clone(),
    };

    element.on_place(closure!(
        [context, registration] | placement | {
            let mut items = context.items.borrow_mut();
            items.retain(|current| current.item != registration.item);
            let index = placement.index.unwrap_or(items.len()).min(items.len());
            items.insert(index, registration.clone());
            drop(items);
            context.assert_unique_values();
            let _ = context
                .sidebar
                .insert_navigation_item(registration.item.clone(), index);
            context.sync_selection();
        }
    ));

    element.on_unmount(closure!(
        [context, item] || {
            context
                .items
                .borrow_mut()
                .retain(|registration| registration.item != item);
            let _ = context.sidebar.remove_navigation_item(&item);
            context.sync_selection();
        }
    ));

    scoped_effect!(
        [context, item, props.label, props.value, props.enabled] || {
            let _ = item.set_label(label.get());
            let _ = item.set_enabled(enabled.get());
            let _ = value.get();
            context.assert_unique_values();
            context.sync_selection();
        }
    );
}
