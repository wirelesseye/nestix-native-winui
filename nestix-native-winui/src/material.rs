use nestix_native_core::{Material, WindowsMaterial};
use windows_core::{Interface, Result};

use crate::bindings::Microsoft::UI::{
    Composition::SystemBackdrops::MicaKind,
    Xaml::Media::{DesktopAcrylicBackdrop, MicaBackdrop, SystemBackdrop},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowBackdropKind {
    Mica,
    MicaAlt,
    Acrylic,
}

pub(crate) fn window_system_backdrop(material: Option<Material>) -> Result<Option<SystemBackdrop>> {
    let Some(kind) = material
        .and_then(Material::windows_material)
        .and_then(window_backdrop_kind)
    else {
        return Ok(None);
    };

    Ok(Some(match kind {
        WindowBackdropKind::Mica => MicaBackdrop::new()?.cast()?,
        WindowBackdropKind::MicaAlt => {
            let backdrop = MicaBackdrop::new()?;
            backdrop.SetKind(MicaKind::BaseAlt)?;
            backdrop.cast()?
        }
        // The XAML DesktopAcrylicBackdrop in the Windows App SDK version used
        // by this crate does not expose the newer Thin kind yet. It still
        // provides the correct system-managed fallback for both acrylic APIs.
        WindowBackdropKind::Acrylic => DesktopAcrylicBackdrop::new()?.cast()?,
    }))
}

pub(crate) fn in_app_brush_resource(material: Material) -> Option<&'static str> {
    match material.windows_material()? {
        WindowsMaterial::Mica | WindowsMaterial::Acrylic => Some("AcrylicInAppFillColorBaseBrush"),
        WindowsMaterial::MicaAlt | WindowsMaterial::AcrylicThin => {
            Some("AcrylicInAppFillColorDefaultBrush")
        }
        _ => None,
    }
}

fn window_backdrop_kind(material: WindowsMaterial) -> Option<WindowBackdropKind> {
    Some(match material {
        WindowsMaterial::Mica => WindowBackdropKind::Mica,
        WindowsMaterial::MicaAlt => WindowBackdropKind::MicaAlt,
        WindowsMaterial::Acrylic | WindowsMaterial::AcrylicThin => WindowBackdropKind::Acrylic,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_current_windows_material_to_a_window_backdrop() {
        assert_eq!(
            window_backdrop_kind(WindowsMaterial::Mica),
            Some(WindowBackdropKind::Mica)
        );
        assert_eq!(
            window_backdrop_kind(WindowsMaterial::MicaAlt),
            Some(WindowBackdropKind::MicaAlt)
        );
        assert_eq!(
            window_backdrop_kind(WindowsMaterial::Acrylic),
            Some(WindowBackdropKind::Acrylic)
        );
        assert_eq!(
            window_backdrop_kind(WindowsMaterial::AcrylicThin),
            Some(WindowBackdropKind::Acrylic)
        );
    }

    #[test]
    fn bounded_materials_use_in_app_acrylic_resources() {
        assert_eq!(
            in_app_brush_resource(Material::WINDOW),
            Some("AcrylicInAppFillColorBaseBrush")
        );
        assert_eq!(
            in_app_brush_resource(Material::SIDEBAR),
            Some("AcrylicInAppFillColorDefaultBrush")
        );
    }
}
