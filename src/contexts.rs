use std::rc::Rc;

use nestix::{Placement, Shared};
use taffy::NodeId;

use crate::{xaml::XamlElement, xaml_app::XamlApp};

type AddChild = Shared<dyn Fn(XamlElement, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(XamlElement, Option<NodeId>, Option<XamlElement>)>;
type RemoveChild = Shared<dyn Fn(&XamlElement, Option<NodeId>)>;

#[derive(Clone)]
pub(crate) struct AppContext {
    pub app: Rc<XamlApp>,
}

#[derive(Clone)]
/// Carries the erased XAML identity used at composition boundaries. Individual
/// components retain typed wrappers, so only parent/child plumbing is type-erased.
pub(crate) struct ParentContext {
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}

impl ParentContext {
    pub fn place_child(
        &self,
        child: XamlElement,
        child_node: Option<NodeId>,
        placement: &Placement,
    ) {
        if let Some(insert_child) = &self.insert_child {
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<XamlElement>().cloned());
            insert_child(child, child_node, predecessor);
        } else if let Some(add_child) = &self.add_child {
            add_child(child, child_node);
        }
    }
}
