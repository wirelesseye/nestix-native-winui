use std::rc::Rc;

use nestix::Shared;

use crate::{xaml::XamlElement, xaml_app::XamlApp};

#[derive(Clone)]
pub(crate) struct AppContext {
    pub app: Rc<XamlApp>,
}

#[derive(Clone)]
pub(crate) struct ParentContext {
    pub add_child: Shared<dyn Fn(XamlElement)>,
    pub remove_child: Shared<dyn Fn(&XamlElement)>,
}
