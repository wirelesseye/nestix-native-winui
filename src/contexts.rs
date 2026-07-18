use std::rc::Rc;

use nestix::{Element, Shared};
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

/// Returns the nearest preceding host handle from the same Nestix list,
/// skipping logical siblings that do not render a XAML element.
pub(crate) fn native_predecessor(element: &Element) -> Option<XamlElement> {
    element
        .previous_siblings()
        .into_iter()
        .find_map(|sibling| sibling.last_handle())
        .and_then(|handle| handle.downcast_ref::<XamlElement>().cloned())
}
