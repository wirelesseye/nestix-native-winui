use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use nestix::{
    Element, PropValue, Shared, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    CheckMenuItemProps, ContextMenuPosition, ContextMenuPresenter, ContextMenuProps,
    ContextMenuRegistration, MenuItemProps, MenuProps, MenuSeparatorProps, RadioMenuItemProps,
    Shortcut, ShortcutKey, ShortcutModifiers, SubmenuProps,
};
use windows::Win32::{
    Foundation::POINT, Graphics::Gdi::ScreenToClient, UI::WindowsAndMessaging::GetCursorPos,
};
use windows_core::{EventRevoker, HSTRING, Interface, Result};

use crate::{
    WindowContext,
    bindings::{
        Microsoft::UI::Xaml::{
            Controls::{
                MenuFlyout as NativeMenuFlyout, MenuFlyoutItem as NativeMenuFlyoutItem,
                MenuFlyoutItemBase, MenuFlyoutSeparator as NativeMenuFlyoutSeparator,
                MenuFlyoutSubItem as NativeMenuFlyoutSubItem,
                Primitives::{FlyoutBase, FlyoutShowOptions},
                RadioMenuFlyoutItem as NativeRadioMenuFlyoutItem,
                ToggleMenuFlyoutItem as NativeToggleMenuFlyoutItem,
            },
            FrameworkElement, UIElement, Visibility,
        },
        Windows::Foundation::Point,
    },
    xaml::XamlElement,
    xaml_app::is_xaml_running,
    xaml_events::RegisteredClickCallback,
};

struct MenuClickHandler {
    _callback: RegisteredClickCallback,
    _revoker: EventRevoker,
}

#[derive(Clone)]
enum NativeEntry {
    Item(NativeMenuFlyoutItem),
    Check(NativeToggleMenuFlyoutItem),
    Radio(NativeRadioMenuFlyoutItem),
    Separator(NativeMenuFlyoutSeparator),
    Submenu(NativeMenuFlyoutSubItem),
}

impl NativeEntry {
    fn base(&self) -> Result<MenuFlyoutItemBase> {
        match self {
            Self::Item(item) => item.cast(),
            Self::Check(item) => item.cast(),
            Self::Radio(item) => item.cast(),
            Self::Separator(item) => item.cast(),
            Self::Submenu(item) => item.cast(),
        }
    }

    fn set_label(&self, label: &str) -> Result<()> {
        let label = HSTRING::from(label);
        match self {
            Self::Item(item) => item.SetText(&label),
            Self::Check(item) => item.SetText(&label),
            Self::Radio(item) => item.SetText(&label),
            Self::Submenu(item) => item.SetText(&label),
            Self::Separator(_) => Ok(()),
        }
    }

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        match self {
            Self::Item(item) => item.SetIsEnabled(enabled),
            Self::Check(item) => item.SetIsEnabled(enabled),
            Self::Radio(item) => item.SetIsEnabled(enabled),
            Self::Submenu(item) => item.SetIsEnabled(enabled),
            Self::Separator(_) => Ok(()),
        }
    }

    fn set_visible(&self, visible: bool) -> Result<()> {
        let visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Collapsed
        };
        match self {
            Self::Item(item) => item.SetVisibility(visibility),
            Self::Check(item) => item.SetVisibility(visibility),
            Self::Radio(item) => item.SetVisibility(visibility),
            Self::Separator(item) => item.SetVisibility(visibility),
            Self::Submenu(item) => item.SetVisibility(visibility),
        }
    }

    fn set_shortcut(&self, shortcut: Option<Shortcut>) -> Result<()> {
        let text = HSTRING::from(shortcut.map(shortcut_text).unwrap_or_default());
        match self {
            Self::Item(item) => item.SetKeyboardAcceleratorTextOverride(&text),
            Self::Check(item) => item.SetKeyboardAcceleratorTextOverride(&text),
            Self::Radio(item) => item.SetKeyboardAcceleratorTextOverride(&text),
            Self::Separator(_) | Self::Submenu(_) => Ok(()),
        }
    }
}

struct RealizedEntry {
    native: NativeEntry,
    _click: Option<MenuClickHandler>,
}

enum EntryKind {
    Item,
    Check,
    Radio,
    Separator,
    Submenu(Rc<MenuData>),
}

struct Entry {
    kind: EntryKind,
    label: RefCell<String>,
    enabled: Cell<bool>,
    visible: Cell<bool>,
    shortcut: Cell<Option<Shortcut>>,
    checked: Cell<bool>,
    group: RefCell<Option<String>>,
    action: Shared<dyn Fn()>,
    realized: RefCell<Option<RealizedEntry>>,
}

impl Entry {
    fn realize(&self) -> Result<()> {
        if self.realized.borrow().is_none() {
            let (native, click_source): (NativeEntry, Option<NativeMenuFlyoutItem>) =
                match &self.kind {
                    EntryKind::Item => {
                        let item = NativeMenuFlyoutItem::new()?;
                        (NativeEntry::Item(item.clone()), Some(item))
                    }
                    EntryKind::Check => {
                        let item = NativeToggleMenuFlyoutItem::new()?;
                        let source = item.cast::<NativeMenuFlyoutItem>()?;
                        (NativeEntry::Check(item), Some(source))
                    }
                    EntryKind::Radio => {
                        let item = NativeRadioMenuFlyoutItem::new()?;
                        let source = item.cast::<NativeMenuFlyoutItem>()?;
                        (NativeEntry::Radio(item), Some(source))
                    }
                    EntryKind::Separator => (
                        NativeEntry::Separator(NativeMenuFlyoutSeparator::new()?),
                        None,
                    ),
                    EntryKind::Submenu(menu) => {
                        menu.rebuild()?;
                        (NativeEntry::Submenu(menu.submenu()?), None)
                    }
                };
            let click = if let Some(source) = click_source {
                let registered = RegisteredClickCallback::register(self.action.clone());
                let callback_id = registered.id();
                let revoker = source.Click(move |_, _| {
                    RegisteredClickCallback::invoke(callback_id);
                })?;
                Some(MenuClickHandler {
                    _callback: registered,
                    _revoker: revoker,
                })
            } else {
                None
            };
            self.realized.replace(Some(RealizedEntry {
                native,
                _click: click,
            }));
        }
        self.update()
    }

    fn update(&self) -> Result<()> {
        let realized = self.realized.borrow();
        let Some(realized) = realized.as_ref() else {
            return Ok(());
        };
        realized.native.set_label(&self.label.borrow())?;
        realized.native.set_enabled(self.enabled.get())?;
        realized.native.set_visible(self.visible.get())?;
        realized.native.set_shortcut(self.shortcut.get())?;
        match &realized.native {
            NativeEntry::Check(item) => item.SetIsChecked(self.checked.get())?,
            NativeEntry::Radio(item) => {
                item.SetIsChecked(self.checked.get())?;
                item.SetGroupName(&HSTRING::from(
                    self.group.borrow().as_deref().unwrap_or_default(),
                ))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn native(&self) -> Result<MenuFlyoutItemBase> {
        self.realize()?;
        self.realized
            .borrow()
            .as_ref()
            .expect("realized menu entry")
            .native
            .base()
    }

    fn native_checked(&self) -> bool {
        self.realized
            .borrow()
            .as_ref()
            .and_then(|entry| match &entry.native {
                NativeEntry::Check(item) => item.IsChecked().ok(),
                NativeEntry::Radio(item) => item.IsChecked().ok(),
                _ => None,
            })
            .unwrap_or(self.checked.get())
    }
}

enum NativeMenu {
    Root(NativeMenuFlyout),
    Submenu(NativeMenuFlyoutSubItem),
}

pub(crate) struct MenuData {
    root: bool,
    native: RefCell<Option<NativeMenu>>,
    entries: RefCell<Vec<Rc<Entry>>>,
}

impl PartialEq for MenuData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MenuData {
    fn new(root: bool) -> Rc<Self> {
        Rc::new(Self {
            root,
            native: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
        })
    }

    fn ensure_native(&self) -> Result<()> {
        if self.native.borrow().is_none() {
            self.native.replace(Some(if self.root {
                NativeMenu::Root(NativeMenuFlyout::new()?)
            } else {
                NativeMenu::Submenu(NativeMenuFlyoutSubItem::new()?)
            }));
        }
        Ok(())
    }

    fn items(&self) -> Result<windows_collections::IVector<MenuFlyoutItemBase>> {
        self.ensure_native()?;
        match self.native.borrow().as_ref().expect("realized menu") {
            NativeMenu::Root(menu) => menu.Items(),
            NativeMenu::Submenu(menu) => menu.Items(),
        }
    }

    fn submenu(&self) -> Result<NativeMenuFlyoutSubItem> {
        self.ensure_native()?;
        match self.native.borrow().as_ref().expect("realized menu") {
            NativeMenu::Submenu(menu) => Ok(menu.clone()),
            NativeMenu::Root(_) => unreachable!("root menu used as submenu"),
        }
    }

    fn flyout(&self) -> Result<NativeMenuFlyout> {
        self.ensure_native()?;
        match self.native.borrow().as_ref().expect("realized menu") {
            NativeMenu::Root(menu) => Ok(menu.clone()),
            NativeMenu::Submenu(_) => unreachable!("submenu used as root menu"),
        }
    }

    fn rebuild(&self) -> Result<()> {
        if !is_xaml_running() {
            return Ok(());
        }
        let items = self.items()?;
        while items.Size()? > 0 {
            items.RemoveAtEnd()?;
        }
        for entry in self.entries.borrow().iter() {
            items.Append(&entry.native()?)?;
        }
        Ok(())
    }

    pub(crate) fn attach(&self, target: &FrameworkElement) -> Result<()> {
        self.rebuild()?;
        let flyout = self.flyout()?.cast::<FlyoutBase>()?;
        target.SetContextFlyout(&flyout)
    }
}

#[derive(Clone)]
struct MenuContext(Rc<MenuData>);

#[derive(Clone)]
struct ContextMenuContext {
    menu: State<Option<Rc<MenuData>>>,
}

fn place_entry(element: &Element, menu: Rc<MenuData>, entry: Rc<Entry>) {
    element.on_place(closure!(
        [menu, entry] | placement | {
            let mut entries = menu.entries.borrow_mut();
            entries.retain(|current| !Rc::ptr_eq(current, &entry));
            let index = placement.index.unwrap_or(entries.len()).min(entries.len());
            entries.insert(index, entry.clone());
            drop(entries);
            let _ = menu.rebuild();
        }
    ));
    element.on_unmount(closure!(
        [menu, entry] || {
            menu.entries
                .borrow_mut()
                .retain(|current| !Rc::ptr_eq(current, &entry));
            let _ = menu.rebuild();
        }
    ));
}

fn common_effects(
    element: &Element,
    entry: Rc<Entry>,
    label: PropValue<String>,
    enabled: PropValue<bool>,
    visible: PropValue<bool>,
    shortcut: PropValue<Option<Shortcut>>,
) {
    scoped_effect!(
        element,
        [entry, label, enabled, visible, shortcut] || {
            *entry.label.borrow_mut() = label.get();
            entry.enabled.set(enabled.get());
            entry.visible.set(visible.get());
            entry.shortcut.set(shortcut.get());
            let _ = entry.update();
        }
    );
}

fn entry(kind: EntryKind, action: Shared<dyn Fn()>) -> Rc<Entry> {
    Rc::new(Entry {
        kind,
        label: RefCell::new(String::new()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        shortcut: Cell::new(None),
        checked: Cell::new(false),
        group: RefCell::new(None),
        action,
        realized: RefCell::new(None),
    })
}

#[component]
pub fn Menu(props: &MenuProps, element: &Element) -> Element {
    let menu = MenuData::new(true);
    if let Some(context) = element.context::<ContextMenuContext>() {
        context.menu.set(Some(menu.clone()));
        element.on_unmount(closure!(
            [context, menu] || {
                if context
                    .menu
                    .get()
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &menu))
                {
                    context.menu.set(None);
                }
            }
        ));
    }
    layout! { ContextProvider<MenuContext>(MenuContext(menu)) { $(props.children.clone()) } }
}

#[component]
pub fn Submenu(props: &SubmenuProps, element: &Element) -> Element {
    let parent = element.context::<MenuContext>().unwrap().0.clone();
    let menu = MenuData::new(false);
    let entry = entry(EntryKind::Submenu(menu.clone()), callback!(|| {}));
    place_entry(element, parent, entry.clone());
    common_effects(
        element,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        PropValue::from_plain(None),
    );
    layout! { ContextProvider<MenuContext>(MenuContext(menu)) { $(props.children.clone()) } }
}

#[component]
pub fn MenuItem(props: &MenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry = entry(
        EntryKind::Item,
        callback!(
            [props.on_activate] || {
                if let Some(action) = on_activate.get() {
                    action();
                }
            }
        ),
    );
    place_entry(element, menu, entry.clone());
    common_effects(
        element,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
}

#[component]
pub fn CheckMenuItem(props: &CheckMenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let slot = Rc::new(RefCell::new(Weak::<Entry>::new()));
    let entry = entry(
        EntryKind::Check,
        callback!(
            [slot, props.on_checked_change] || {
                if let Some(entry) = slot.borrow().upgrade() {
                    let checked = entry.native_checked();
                    entry.checked.set(checked);
                    if let Some(action) = on_checked_change.get() {
                        action(checked);
                    }
                }
            }
        ),
    );
    *slot.borrow_mut() = Rc::downgrade(&entry);
    place_entry(element, menu, entry.clone());
    common_effects(
        element,
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [entry, props.checked] || {
            entry.checked.set(checked.get());
            let _ = entry.update();
        }
    );
}

#[component]
pub fn RadioMenuItem(props: &RadioMenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let menu_slot = Rc::downgrade(&menu);
    let entry_slot = Rc::new(RefCell::new(Weak::<Entry>::new()));
    let entry = entry(
        EntryKind::Radio,
        callback!(
            [menu_slot, entry_slot, props.group, props.on_select] || {
                if let (Some(menu), Some(selected)) =
                    (menu_slot.upgrade(), entry_slot.borrow().upgrade())
                {
                    for item in menu.entries.borrow().iter() {
                        if item.group.borrow().as_deref() == Some(group.get().as_str()) {
                            item.checked.set(Rc::ptr_eq(item, &selected));
                            let _ = item.update();
                        }
                    }
                    if let Some(action) = on_select.get() {
                        action();
                    }
                }
            }
        ),
    );
    *entry_slot.borrow_mut() = Rc::downgrade(&entry);
    place_entry(element, menu, entry.clone());
    common_effects(
        element,
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [entry, props.selected] || {
            entry.checked.set(selected.get());
            let _ = entry.update();
        }
    );
    scoped_effect!(
        element,
        [entry, props.group] || {
            *entry.group.borrow_mut() = Some(group.get());
            let _ = entry.update();
        }
    );
}

#[component]
pub fn MenuSeparator(props: &MenuSeparatorProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry = entry(EntryKind::Separator, callback!(|| {}));
    place_entry(element, menu, entry.clone());
    scoped_effect!(
        element,
        [entry, props.visible] || {
            entry.visible.set(visible.get());
            let _ = entry.update();
        }
    );
}

fn show_menu(
    menu: &MenuData,
    target: &XamlElement,
    window: &WindowContext,
    position: ContextMenuPosition,
) -> bool {
    if !is_xaml_running() || menu.rebuild().is_err() {
        return false;
    }
    let (Ok(menu), Ok(target)) = (menu.flyout(), target.as_framework_element()) else {
        return false;
    };
    let result = match position {
        ContextMenuPosition::Anchor => menu.ShowAt(&target),
        ContextMenuPosition::Point(position) => show_menu_at_point(
            &menu,
            &target,
            Point {
                X: position.x as f32,
                Y: position.y as f32,
            },
        ),
        ContextMenuPosition::Cursor => {
            let Ok(hwnd) = window.window.hwnd() else {
                return false;
            };
            let mut cursor = POINT::default();
            if unsafe { GetCursorPos(&mut cursor) }.is_err()
                || !unsafe { ScreenToClient(hwnd, &mut cursor) }.as_bool()
            {
                return false;
            }
            let scale = window.scale_factor.get() as f32;
            let origin = target
                .TransformToVisual(None::<&UIElement>)
                .and_then(|transform| transform.TransformPoint(Point { X: 0.0, Y: 0.0 }));
            let Ok(origin) = origin else {
                return false;
            };
            show_menu_at_point(
                &menu,
                &target,
                Point {
                    X: cursor.x as f32 / scale - origin.X,
                    Y: cursor.y as f32 / scale - origin.Y,
                },
            )
        }
    };
    result.is_ok()
}

fn show_menu_at_point(
    menu: &NativeMenuFlyout,
    target: &FrameworkElement,
    point: Point,
) -> Result<()> {
    let options = FlyoutShowOptions::new()?;
    options.SetPosition(Some(point))?;
    menu.ShowAtWithOptions(target, &options)
}

#[component]
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let window = element.context::<WindowContext>().unwrap();
    let menu = create_state(None::<Rc<MenuData>>);
    let target = create_state(None::<XamlElement>);
    let context = Rc::new(ContextMenuContext { menu: menu.clone() });
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));
    let attached = Rc::new(RefCell::new(None::<(XamlElement, Rc<MenuData>)>));

    scoped_effect!(
        element,
        [target, props.children] || {
            children.get().on_last_handle_change(closure!(
                [target] | handle | {
                    target
                        .set(handle.and_then(|value| value.downcast_ref::<XamlElement>().cloned()));
                }
            ));
        }
    );

    scoped_effect!(
        element,
        [
            window,
            menu,
            target,
            props.controller,
            registration,
            attached
        ] || {
            registration.borrow_mut().take();
            if let Some((old_target, old_menu)) = attached.borrow_mut().take() {
                let _ = old_target.clear_context_menu_if(&old_menu);
            }
            let (Some(menu), Some(target)) = (menu.get(), target.get()) else {
                return;
            };
            if target.set_context_menu(Some(menu.clone())).is_err() {
                return;
            }
            attached
                .borrow_mut()
                .replace((target.clone(), menu.clone()));
            if let Some(controller) = controller.get() {
                registration
                    .borrow_mut()
                    .replace(controller.bind(ContextMenuPresenter {
                        show: callback!(
                            [menu, target, window] | position | {
                                show_menu(&menu, &target, &window, position)
                            }
                        ),
                        dismiss: callback!(
                            [menu] || {
                                if is_xaml_running()
                                    && let Ok(menu) = menu.flyout()
                                {
                                    let _ = menu.Hide();
                                }
                            }
                        ),
                    }));
            }
        }
    );

    element.on_unmount(closure!(
        [registration, attached] || {
            registration.borrow_mut().take();
            if let Some((target, menu)) = attached.borrow_mut().take() {
                let _ = target.clear_context_menu_if(&menu);
            }
        }
    ));

    layout! {
        ContextProvider<ContextMenuContext>(context) [props.children, props.menu] {
            yield $(children.get())
            yield $(menu.get())
        }
    }
}

fn shortcut_text(shortcut: Shortcut) -> String {
    let mut text = String::new();
    let modifiers = shortcut.modifiers();
    if modifiers.contains(ShortcutModifiers::PRIMARY) {
        text.push_str("Ctrl+");
    }
    if modifiers.contains(ShortcutModifiers::SHIFT) {
        text.push_str("Shift+");
    }
    if modifiers.contains(ShortcutModifiers::ALT) {
        text.push_str("Alt+");
    }
    text.push_str(&match shortcut.key() {
        ShortcutKey::Character(value) => value.to_ascii_uppercase().to_string(),
        ShortcutKey::Backspace => "Backspace".into(),
        ShortcutKey::Delete => "Del".into(),
        ShortcutKey::Down => "Down".into(),
        ShortcutKey::End => "End".into(),
        ShortcutKey::Enter => "Enter".into(),
        ShortcutKey::Escape => "Esc".into(),
        ShortcutKey::Home => "Home".into(),
        ShortcutKey::Insert => "Ins".into(),
        ShortcutKey::Left => "Left".into(),
        ShortcutKey::PageDown => "PgDn".into(),
        ShortcutKey::PageUp => "PgUp".into(),
        ShortcutKey::Right => "Right".into(),
        ShortcutKey::Tab => "Tab".into(),
        ShortcutKey::Up => "Up".into(),
        ShortcutKey::Function(number) => format!("F{number}"),
    });
    text
}

#[cfg(test)]
mod tests {
    use super::{MenuData, shortcut_text};
    use nestix_native_core::{Shortcut, ShortcutKey, ShortcutModifiers};

    #[test]
    fn menu_construction_is_deferred_until_xaml_is_running() {
        let menu = MenuData::new(true);
        assert!(menu.native.borrow().is_none());
        menu.rebuild().unwrap();
        assert!(menu.native.borrow().is_none());
    }

    #[test]
    fn formats_windows_shortcuts() {
        let shortcut = Shortcut::new(
            ShortcutKey::Character('s'),
            ShortcutModifiers::PRIMARY | ShortcutModifiers::SHIFT,
        )
        .unwrap();
        assert_eq!(shortcut_text(shortcut), "Ctrl+Shift+S");
    }
}
