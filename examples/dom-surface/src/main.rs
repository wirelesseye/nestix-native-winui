mod app;

fn main() {
    use env_logger::Env;
    use nestix::{layout, mount_root};
    use nestix_native_winui::WINUI_BACKEND;

    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {
        nestix::ContextProvider<nestix_native::BackendContext>(
            nestix_native::BackendContext { backend: &WINUI_BACKEND },
        ) {
            app::App
        }
    });
}
