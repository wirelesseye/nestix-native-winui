use std::rc::Rc;

use nestix::Shared;
use taffy::NodeId;

use crate::{xaml::XamlElement, xaml_app::XamlApp};

type AddChild = Shared<dyn Fn(XamlElement, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(XamlElement, Option<NodeId>, usize)>;
type RemoveChild = Shared<dyn Fn(&XamlElement, Option<NodeId>)>;

#[derive(Clone)]
pub(crate) struct AppContext {
    pub app: Rc<XamlApp>,
}

#[derive(Clone)]
pub(crate) struct ParentContext {
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}
