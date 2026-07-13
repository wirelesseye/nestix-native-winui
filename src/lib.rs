mod app_shim;
pub mod button;
mod contexts;
pub mod flex_view;
pub mod image_view;
pub mod input;
pub mod root;
pub mod scroll_view;
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
pub use flex_view::*;
pub use image_view::*;
pub use input::*;
pub use root::*;
pub use scroll_view::*;
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
}
