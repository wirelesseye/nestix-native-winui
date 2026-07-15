mod app_shim;
pub mod button;
pub mod checkbox;
mod contexts;
pub mod flex_view;
pub mod image_view;
pub mod input;
pub mod menu;
mod native_control;
pub mod radio_button;
pub mod root;
pub mod scroll_view;
pub mod select;
pub mod slider;
pub mod switch;
pub mod tab_view;
pub mod text;
pub mod window;
mod window_native;
mod xaml;
mod xaml_app;
mod xaml_events;

#[allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use button::*;
pub use checkbox::*;
pub use flex_view::*;
pub use image_view::*;
pub use input::*;
pub use menu::*;
pub use radio_button::*;
pub use root::*;
pub use scroll_view::*;
pub use select::*;
pub use slider::*;
pub use switch::*;
pub use tab_view::*;
pub use text::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

pub const WINUI_BACKEND: WinUiBackend = WinUiBackend;

pub struct WinUiBackend;

impl Backend for WinUiBackend {
    fn backend_id(&self) -> &'static str {
        "nestix-native-winui"
    }

    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<nestix::Element> {
        Some(create_element::<Root>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        Some(create_element::<Button>(props))
    }

    fn create_checkbox(&self, props: nestix_native_core::CheckboxProps) -> Option<nestix::Element> {
        Some(create_element::<Checkbox>(props))
    }

    fn create_radio_button(
        &self,
        props: nestix_native_core::RadioButtonProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<RadioButton>(props))
    }

    fn create_select(&self, props: nestix_native_core::SelectProps) -> Option<nestix::Element> {
        Some(create_element::<Select>(props))
    }

    fn create_select_option(
        &self,
        props: nestix_native_core::SelectOptionProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<SelectOption>(props))
    }

    fn create_switch(&self, props: nestix_native_core::SwitchProps) -> Option<nestix::Element> {
        Some(create_element::<Switch>(props))
    }

    fn create_slider(&self, props: nestix_native_core::SliderProps) -> Option<nestix::Element> {
        Some(create_element::<Slider>(props))
    }

    fn create_flex_view(
        &self,
        props: nestix_native_core::FlexViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<FlexView>(props))
    }

    fn create_input(&self, props: nestix_native_core::InputProps) -> Option<nestix::Element> {
        Some(create_element::<Input>(props))
    }

    fn create_image_view(
        &self,
        props: nestix_native_core::ImageViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ImageView>(props))
    }

    fn create_scroll_view(
        &self,
        props: nestix_native_core::ScrollViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ScrollView>(props))
    }

    fn create_text(&self, props: nestix_native_core::TextProps) -> Option<nestix::Element> {
        Some(create_element::<Text>(props))
    }

    fn create_tab_view(&self, props: nestix_native_core::TabViewProps) -> Option<nestix::Element> {
        Some(create_element::<TabView>(props))
    }

    fn create_tab_view_item(
        &self,
        props: nestix_native_core::TabViewItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<TabViewItem>(props))
    }

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<Window>(props))
    }

    fn create_menu(&self, props: nestix_native_core::MenuProps) -> Option<nestix::Element> {
        Some(create_element::<Menu>(props))
    }

    fn create_menu_bar(&self, props: nestix_native_core::MenuBarProps) -> Option<nestix::Element> {
        Some(create_element::<MenuBar>(props))
    }

    fn create_submenu(&self, props: nestix_native_core::SubmenuProps) -> Option<nestix::Element> {
        Some(create_element::<Submenu>(props))
    }

    fn create_menu_item(
        &self,
        props: nestix_native_core::MenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<MenuItem>(props))
    }

    fn create_check_menu_item(
        &self,
        props: nestix_native_core::CheckMenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<CheckMenuItem>(props))
    }

    fn create_radio_menu_item(
        &self,
        props: nestix_native_core::RadioMenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<RadioMenuItem>(props))
    }

    fn create_menu_separator(
        &self,
        props: nestix_native_core::MenuSeparatorProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<MenuSeparator>(props))
    }

    fn create_context_menu(
        &self,
        props: nestix_native_core::ContextMenuProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ContextMenu>(props))
    }
}
