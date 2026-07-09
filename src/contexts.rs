use std::rc::Rc;

use nestix::Shared;

use crate::xaml::XamlElement;

#[derive(Clone)]
pub(crate) struct AppContext {
    pub app: Rc<crate::xaml::XamlApp>,
}

#[derive(Clone)]
pub(crate) struct ParentContext {
    pub add_child: Shared<dyn Fn(XamlElement)>,
    pub remove_child: Shared<dyn Fn(&XamlElement)>,
}
