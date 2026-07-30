use nestix::{ContextProvider, layout, mount_root, unmount_root};
use nestix_native_core::NativeVisualMount;
use nestix_native_winui::Button;

#[test]
fn direct_visual_component_stops_at_blocked_boundary() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::blocked("test boundary")) {
            Button(.title = "Blocked")
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}

#[test]
fn direct_visual_component_stops_at_foreign_visual_tree() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::allowed("another-backend")) {
            Button(.title = "Foreign")
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}
