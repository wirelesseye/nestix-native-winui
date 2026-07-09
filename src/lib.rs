mod app_shim;
pub mod button;
mod contexts;
pub mod flex_view;
pub mod root;
pub mod text;
pub mod window;
mod xaml;

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
pub use root::*;
pub use text::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

pub struct WinUiBackend;

impl Backend for WinUiBackend {
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

    fn create_input(&self, _props: nestix_native_core::InputProps) -> Option<nestix::Element> {
        None
    }

    fn create_text(&self, props: nestix_native_core::TextProps) -> Option<nestix::Element> {
        Some(create_element::<Text>(props))
    }

    fn create_tab_view(&self, _props: nestix_native_core::TabViewProps) -> Option<nestix::Element> {
        None
    }

    fn create_tab_view_item(
        &self,
        _props: nestix_native_core::TabViewItemProps,
    ) -> Option<nestix::Element> {
        None
    }

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<Window>(props))
    }
}
