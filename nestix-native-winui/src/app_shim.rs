use std::cell::RefCell;

use windows_core::{Array, HSTRING, Ref, Result, implement};

use crate::bindings::{
    Microsoft::UI::Xaml::{
        Application, IApplicationOverrides, IApplicationOverrides_Impl, LaunchActivatedEventArgs,
        Markup::{IXamlMetadataProvider, IXamlMetadataProvider_Impl, IXamlType, XmlnsDefinition},
        XamlTypeInfo::XamlControlsXamlMetaDataProvider,
    },
    Windows::UI::Xaml::Interop::TypeName,
};

#[implement(IApplicationOverrides, IXamlMetadataProvider)]
pub struct XamlApplicationOverrides {
    controls_provider: RefCell<Option<XamlControlsXamlMetaDataProvider>>,
    on_launched: RefCell<Option<Box<dyn FnOnce() -> Result<()>>>>,
}

impl XamlApplicationOverrides {
    fn new(on_launched: Box<dyn FnOnce() -> Result<()>>) -> Self {
        Self {
            controls_provider: RefCell::new(None),
            on_launched: RefCell::new(Some(on_launched)),
        }
    }

    fn provider(&self) -> Result<XamlControlsXamlMetaDataProvider> {
        if let Some(provider) = self.controls_provider.borrow().as_ref() {
            return Ok(provider.clone());
        }

        let provider = XamlControlsXamlMetaDataProvider::new()?;
        self.controls_provider.replace(Some(provider.clone()));
        Ok(provider)
    }
}

impl IApplicationOverrides_Impl for XamlApplicationOverrides_Impl {
    fn OnLaunched(&self, _args: Ref<LaunchActivatedEventArgs>) -> Result<()> {
        if let Some(on_launched) = self.on_launched.borrow_mut().take() {
            on_launched()?;
        }
        Ok(())
    }
}

impl IXamlMetadataProvider_Impl for XamlApplicationOverrides_Impl {
    fn GetXamlType(&self, r#type: &TypeName) -> Result<IXamlType> {
        self.provider()?.GetXamlType(r#type)
    }

    fn GetXamlTypeByFullName(&self, full_name: &HSTRING) -> Result<IXamlType> {
        self.provider()?.GetXamlTypeByFullName(full_name)
    }

    fn GetXmlnsDefinitions(&self) -> Result<Array<XmlnsDefinition>> {
        self.provider()?.GetXmlnsDefinitions()
    }
}

pub struct CreatedXamlApplication {
    _application: Application,
}

pub fn create_xaml_application(
    on_launched: Box<dyn FnOnce() -> Result<()>>,
) -> Result<CreatedXamlApplication> {
    Application::compose(XamlApplicationOverrides::new(on_launched)).map(|application| {
        CreatedXamlApplication {
            _application: application,
        }
    })
}
