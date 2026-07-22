use std::{env, fs, path::Path};

const WINDOWS_APP_SDK_WINUI_VERSION: &str = "1.8.260528001";
const WINDOWS_APP_SDK_FOUNDATION_VERSION: &str = "1.8.260527000";
const WINDOWS_APP_SDK_INTERACTIVE_EXPERIENCES_VERSION: &str = "1.8.260525001";

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = Path::new(&manifest_dir);
    let packages_dir = workspace_dir.join(".packages");
    let foundation_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.Foundation")
        .join(WINDOWS_APP_SDK_FOUNDATION_VERSION);
    let winui_metadata_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.WinUI")
        .join(WINDOWS_APP_SDK_WINUI_VERSION)
        .join("metadata");
    let foundation_metadata_dir = foundation_dir.join("metadata");
    let ixp_metadata_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.InteractiveExperiences")
        .join(WINDOWS_APP_SDK_INTERACTIVE_EXPERIENCES_VERSION)
        .join("metadata")
        .join("10.0.18362.0");

    let mut metadata_files = Vec::new();
    collect_winmds(&winui_metadata_dir, &mut metadata_files);
    collect_winmds(&foundation_metadata_dir, &mut metadata_files);
    collect_winmds(&ixp_metadata_dir, &mut metadata_files);

    if metadata_files.is_empty() {
        panic!(
            "Windows App SDK metadata not found under {}. Run scripts\\fetch-windows-app-sdk.ps1 from the nestix-native-winui crate root, then build again.",
            packages_dir.display(),
        );
    }

    for file in &metadata_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("bindings.rs");
    let out = out.to_string_lossy().into_owned();

    let mut args = vec!["--in".to_string(), "default".to_string()];
    args.extend(
        metadata_files
            .iter()
            .map(|file| file.to_string_lossy().into_owned()),
    );
    args.extend([
        "--out".to_string(),
        out,
        "--implement".to_string(),
        "--filter".to_string(),
        "Microsoft.UI.Xaml.Application".to_string(),
        "Microsoft.UI.Xaml.ApplicationInitializationCallback".to_string(),
        "Microsoft.UI.Xaml.DependencyObject".to_string(),
        "Microsoft.UI.Xaml.DependencyProperty".to_string(),
        "Microsoft.UI.Xaml.DragEventArgs".to_string(),
        "Microsoft.UI.Xaml.DragEventHandler".to_string(),
        "Windows.ApplicationModel.DataTransfer.DragDrop.DragDropModifiers".to_string(),
        "Microsoft.UI.Xaml.DragOperationDeferral".to_string(),
        "Microsoft.UI.Xaml.DragStartingEventArgs".to_string(),
        "Microsoft.UI.Xaml.DropCompletedEventArgs".to_string(),
        "Microsoft.UI.Xaml.IApplicationOverrides".to_string(),
        "Microsoft.UI.Xaml.LaunchActivatedEventArgs".to_string(),
        "Microsoft.UI.Xaml.ResourceDictionary".to_string(),
        "Microsoft.UI.Windowing.AppWindow".to_string(),
        "Microsoft.UI.Windowing.AppWindowClosingEventArgs".to_string(),
        "Microsoft.UI.Windowing.AppWindowPresenter".to_string(),
        "Microsoft.UI.Windowing.OverlappedPresenter".to_string(),
        "Microsoft.UI.WindowId".to_string(),
        "Microsoft.UI.Dispatching.DispatcherQueue".to_string(),
        "Microsoft.UI.Dispatching.DispatcherQueueHandler".to_string(),
        "Microsoft.Windows.Storage.Pickers.FileOpenPicker".to_string(),
        "Microsoft.Windows.Storage.Pickers.FileSavePicker".to_string(),
        "Microsoft.Windows.Storage.Pickers.FolderPicker".to_string(),
        "Microsoft.Windows.Storage.Pickers.PickFileResult".to_string(),
        "Microsoft.Windows.Storage.Pickers.PickFolderResult".to_string(),
        "Microsoft.UI.Xaml.Window".to_string(),
        "Microsoft.UI.Xaml.Thickness".to_string(),
        "Microsoft.UI.Xaml.HorizontalAlignment".to_string(),
        "Microsoft.UI.Xaml.Input.KeyboardAccelerator".to_string(),
        "Microsoft.UI.Xaml.Input.KeyboardAcceleratorInvokedEventArgs".to_string(),
        "Microsoft.UI.Xaml.GridLength".to_string(),
        "Microsoft.UI.Xaml.GridUnitType".to_string(),
        "Microsoft.UI.Xaml.VerticalAlignment".to_string(),
        "Microsoft.UI.Xaml.Controls.Button".to_string(),
        "Microsoft.UI.Xaml.Controls.CheckBox".to_string(),
        "Microsoft.UI.Xaml.Controls.ComboBox".to_string(),
        "Microsoft.UI.Xaml.Controls.ComboBoxItem".to_string(),
        "Microsoft.UI.Xaml.Controls.Canvas".to_string(),
        "Microsoft.UI.Xaml.Controls.ContentControl".to_string(),
        "Microsoft.UI.Xaml.Controls.Control".to_string(),
        "Microsoft.UI.Xaml.Controls.ItemsControl".to_string(),
        "Microsoft.UI.Xaml.Controls.IItemsControl".to_string(),
        "Microsoft.UI.Xaml.Controls.ItemCollection".to_string(),
        "Microsoft.UI.Xaml.Controls.Grid".to_string(),
        "Microsoft.UI.Xaml.Controls.Image".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuFlyout".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuBar".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuBarItem".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuFlyoutItem".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuFlyoutSeparator".to_string(),
        "Microsoft.UI.Xaml.Controls.MenuFlyoutSubItem".to_string(),
        "Microsoft.UI.Xaml.Controls.RadioMenuFlyoutItem".to_string(),
        "Microsoft.UI.Xaml.Controls.RadioButton".to_string(),
        "Microsoft.UI.Xaml.Controls.SelectionChangedEventArgs".to_string(),
        "Microsoft.UI.Xaml.Controls.SelectionChangedEventHandler".to_string(),
        "Microsoft.UI.Xaml.Controls.Slider".to_string(),
        "Microsoft.UI.Xaml.Controls.ToggleMenuFlyoutItem".to_string(),
        "Microsoft.UI.Xaml.Controls.ToggleSwitch".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventHandler".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.RangeBase".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.IRangeBase".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.FlyoutPlacementMode".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.FlyoutShowOptions".to_string(),
        "Microsoft.UI.Xaml.Controls.RowDefinition".to_string(),
        "Microsoft.UI.Xaml.Controls.RowDefinitionCollection".to_string(),
        "Microsoft.UI.Xaml.Controls.ScrollView".to_string(),
        "Microsoft.UI.Xaml.Controls.ScrollingContentOrientation".to_string(),
        "Microsoft.UI.Xaml.Controls.ScrollingScrollBarVisibility".to_string(),
        "Microsoft.UI.Xaml.Controls.XamlControlsResources".to_string(),
        "Microsoft.UI.Xaml.Controls.Panel".to_string(),
        "Microsoft.UI.Xaml.Controls.SelectorBar".to_string(),
        "Microsoft.UI.Xaml.Controls.SelectorBarItem".to_string(),
        "Microsoft.UI.Xaml.Controls.SelectorBarSelectionChangedEventArgs".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.ButtonBase".to_string(),
        "Microsoft.UI.Xaml.Controls.TextBlock".to_string(),
        "Microsoft.UI.Xaml.Controls.TextBox".to_string(),
        "Microsoft.UI.Xaml.Controls.TextChangedEventArgs".to_string(),
        "Microsoft.UI.Xaml.Controls.TextChangedEventHandler".to_string(),
        "Microsoft.UI.Xaml.Controls.UIElementCollection".to_string(),
        "Microsoft.UI.Xaml.Media.Brush".to_string(),
        "Microsoft.UI.Xaml.Media.FontFamily".to_string(),
        "Microsoft.UI.Xaml.Media.GeneralTransform".to_string(),
        "Microsoft.UI.Xaml.Media.SolidColorBrush".to_string(),
        "Microsoft.UI.Xaml.Media.Stretch".to_string(),
        "Microsoft.UI.Xaml.Media.Imaging.BitmapImage".to_string(),
        "Microsoft.UI.Xaml.Media.Imaging.BitmapSource".to_string(),
        "Windows.Storage.Streams.IRandomAccessStream".to_string(),
        "Windows.Storage.Streams.IRandomAccessStreamWithContentType".to_string(),
        "Windows.ApplicationModel.DataTransfer.DataPackage".to_string(),
        "Windows.ApplicationModel.DataTransfer.DataPackageOperation".to_string(),
        "Windows.ApplicationModel.DataTransfer.DataPackageView".to_string(),
        "Windows.ApplicationModel.DataTransfer.StandardDataFormats".to_string(),
        "Windows.Storage.IStorageItem".to_string(),
        "Windows.Storage.StorageFile".to_string(),
        "Windows.Storage.Streams.RandomAccessStreamReference".to_string(),
        "Windows.System.VirtualKey".to_string(),
        "Windows.System.VirtualKeyModifiers".to_string(),
        "Microsoft.UI.Xaml.UIElement".to_string(),
        "Microsoft.UI.Xaml.Visibility".to_string(),
        "Microsoft.UI.Xaml.FrameworkElement".to_string(),
        "Microsoft.UI.Xaml.SizeChangedEventArgs".to_string(),
        "Microsoft.UI.Xaml.SizeChangedEventHandler".to_string(),
        "Microsoft.UI.Xaml.Markup.IXamlMetadataProvider".to_string(),
        "Microsoft.UI.Xaml.Markup.IXamlType".to_string(),
        "Microsoft.UI.Xaml.Markup.XmlnsDefinition".to_string(),
        "Microsoft.UI.Xaml.RoutedEventArgs".to_string(),
        "Microsoft.UI.Xaml.RoutedEventHandler".to_string(),
        "Microsoft.UI.Xaml.XamlTypeInfo.XamlControlsXamlMetaDataProvider".to_string(),
        "Windows.Foundation.PropertyValue".to_string(),
        "Windows.Foundation.Deferral".to_string(),
        "Windows.Foundation.Point".to_string(),
        "Windows.Foundation.Size".to_string(),
        "Windows.Foundation.TypedEventHandler".to_string(),
        "Windows.Graphics.SizeInt32".to_string(),
        "Windows.UI.Color".to_string(),
        "Windows.UI.Text.FontStyle".to_string(),
        "Windows.UI.Text.FontWeight".to_string(),
        "Windows.UI.Xaml.Interop.TypeName".to_string(),
    ]);

    windows_bindgen::bindgen(args);

    link_windows_app_runtime_bootstrap(&foundation_dir);
}

fn collect_winmds(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_winmds(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "winmd")
        {
            files.push(path);
        }
    }
}

fn link_windows_app_runtime_bootstrap(foundation_dir: &Path) {
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86" => "x86",
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unsupported Windows App SDK target architecture: {other}"),
    };
    let lib_dir = foundation_dir.join("lib").join("native").join(arch);
    let runtime_dir = foundation_dir
        .join("runtimes")
        .join(format!("win-{arch}"))
        .join("native");
    let bootstrap_dll = runtime_dir.join("Microsoft.WindowsAppRuntime.Bootstrap.dll");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=Microsoft.WindowsAppRuntime.Bootstrap");
    println!("cargo:rerun-if-changed={}", bootstrap_dll.display());
    println!("cargo:rerun-if-changed=assets/resources.pri");

    if let Some(target_dir) = target_profile_dir() {
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(
            &bootstrap_dll,
            target_dir.join("Microsoft.WindowsAppRuntime.Bootstrap.dll"),
        )
        .unwrap_or_else(|err| {
            panic!(
                "failed to copy {} to {}: {err}",
                bootstrap_dll.display(),
                target_dir.display()
            )
        });
        fs::write(
            target_dir.join("resources.pri"),
            include_bytes!("assets/resources.pri"),
        )
        .unwrap_or_else(|err| {
            panic!(
                "failed to write resources.pri to {}: {err}",
                target_dir.display()
            )
        });
    }
}

fn target_profile_dir() -> Option<std::path::PathBuf> {
    let out_dir = std::path::PathBuf::from(env::var("OUT_DIR").ok()?);
    let profile = env::var("PROFILE").ok()?;
    let mut ancestors = out_dir.ancestors();
    while let Some(path) = ancestors.next() {
        if path
            .file_name()
            .is_some_and(|name| name == profile.as_str())
        {
            return Some(path.to_path_buf());
        }
    }
    None
}
