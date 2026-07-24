use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const PACKAGES: &[(&str, &str)] = &[
    ("Microsoft.WindowsAppSDK", "1.8.260529003"),
    ("Microsoft.WindowsAppSDK.Runtime", "1.8.260529003"),
    ("Microsoft.WindowsAppSDK.WinUI", "1.8.260528001"),
    ("Microsoft.WindowsAppSDK.Foundation", "1.8.260527000"),
    ("Microsoft.WindowsAppSDK.Base", "1.8.251216001"),
    (
        "Microsoft.WindowsAppSDK.InteractiveExperiences",
        "1.8.260525001",
    ),
];

const RUNTIME_PACKAGE: (&str, &str) = ("Microsoft.WindowsAppSDK.Runtime", "1.8.260529003");

/// Configures the final application executable for unpackaged, self-contained
/// Windows App SDK deployment.
///
/// Call this from the application's `build.rs`.
pub fn configure() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=NESTIX_WINDOWS_APP_SDK_PACKAGES");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    let arch = target_arch();
    let packages = windows_app_sdk_package_root();
    let output = target_profile_dir().unwrap_or_else(|| {
        panic!("could not determine Cargo's target profile directory from OUT_DIR")
    });

    stage_runtime(&packages, &output, arch);
    embed_manifest();
}

/// Locates the pinned Windows App SDK component packages, downloading them to
/// a user cache when this repository's development cache is unavailable.
pub fn windows_app_sdk_package_root() -> PathBuf {
    if let Some(path) = env::var_os("NESTIX_WINDOWS_APP_SDK_PACKAGES") {
        let path = PathBuf::from(path);
        validate_package_root(&path);
        return path;
    }

    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("build support crate has no parent directory");
    let development_packages = source_root.join(".packages");
    if package_root_is_complete(&development_packages) {
        return development_packages;
    }

    let cache_root = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Nestix")
        .join("WindowsAppSDK")
        .join("packages");
    if !package_root_is_complete(&cache_root) {
        fetch_packages(&cache_root);
    }
    validate_package_root(&cache_root);
    cache_root
}

fn target_arch() -> &'static str {
    match env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap_or_default()
        .as_str()
    {
        "x86" => "x86",
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unsupported Windows App SDK target architecture: {other}"),
    }
}

fn stage_runtime(packages: &Path, output: &Path, arch: &str) {
    fs::create_dir_all(output)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", output.display()));

    let package = packages.join(RUNTIME_PACKAGE.0).join(RUNTIME_PACKAGE.1);
    let msix_dir = package
        .join("tools")
        .join("MSIX")
        .join(format!("win10-{arch}"));
    let msix = fs::read_dir(&msix_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", msix_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with("Microsoft.WindowsAppRuntime.")
                    && !name.contains("DDLM")
                    && !name.contains("Main")
                    && !name.contains("Singleton")
                    && name.ends_with(".msix")
            })
        })
        .unwrap_or_else(|| panic!("Windows App Runtime framework MSIX not found"));
    println!("cargo:rerun-if-changed={}", msix.display());
    let extract = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"))
        .join(format!("windows-app-runtime-{arch}"));
    ensure_msix_extracted(&msix, &extract);
    copy_runtime_payload(&extract, output);
}

fn ensure_msix_extracted(msix: &Path, destination: &Path) {
    let marker = destination.join(".nestix-extracted");
    if marker.is_file() {
        return;
    }
    fs::create_dir_all(destination)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", destination.display()));
    let tar = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("tar.exe"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("tar.exe"));
    let status = Command::new(tar)
        .arg("-xf")
        .arg(msix)
        .arg("-C")
        .arg(destination)
        .status()
        .unwrap_or_else(|err| panic!("failed to extract {}: {err}", msix.display()));
    if !status.success() {
        panic!("failed to extract {}", msix.display());
    }
    fs::write(marker, b"")
        .unwrap_or_else(|err| panic!("failed to mark runtime extraction complete: {err}"));
}

fn copy_runtime_payload(source: &Path, destination: &Path) {
    let entries = fs::read_dir(source)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to enumerate {}: {err}", source.display()));
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path).unwrap_or_else(|err| {
                panic!("failed to create {}: {err}", destination_path.display())
            });
            copy_runtime_payload(&source_path, &destination_path);
        } else if is_runtime_payload_file(&source_path) {
            copy_file_if_changed(source_path, destination_path);
        }
    }
}

fn is_runtime_payload_file(path: &Path) -> bool {
    if path.file_name().is_some_and(|name| {
        name.eq_ignore_ascii_case("RestartAgent.exe") || name.eq_ignore_ascii_case("map.html")
    }) {
        return true;
    }
    path.extension().is_some_and(|extension| {
        matches!(
            extension.to_string_lossy().to_ascii_lowercase().as_str(),
            "dll" | "pri" | "winmd" | "xaml" | "xbf" | "mui" | "png" | "json"
        )
    })
}

fn copy_file_if_changed(source: impl AsRef<Path>, destination: impl AsRef<Path>) {
    let source = source.as_ref();
    let destination = destination.as_ref();
    let unchanged = fs::metadata(source)
        .and_then(|source_metadata| {
            fs::metadata(destination).map(|destination_metadata| {
                source_metadata.len() == destination_metadata.len()
                    && source_metadata.modified().ok() == destination_metadata.modified().ok()
            })
        })
        .unwrap_or(false);
    if unchanged {
        return;
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("failed to create {}: {err}", parent.display()));
    }
    fs::copy(source, destination).unwrap_or_else(|err| {
        panic!(
            "failed to copy {} to {}: {err}",
            source.display(),
            destination.display()
        )
    });
}

fn embed_manifest() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let manifest = out_dir.join("nestix-native-winui.app.manifest");
    fs::write(&manifest, include_bytes!("../app.manifest")).unwrap_or_else(|err| {
        panic!(
            "failed to write {} for WinUI manifest embedding: {err}",
            manifest.display()
        )
    });

    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
        manifest.display()
    );
}

fn target_profile_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?);
    let profile = env::var_os("PROFILE")?;
    out_dir
        .ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == profile))
        .map(Path::to_path_buf)
}

fn package_root_is_complete(root: &Path) -> bool {
    PACKAGES
        .iter()
        .all(|(id, version)| root.join(id).join(version).is_dir())
}

fn validate_package_root(root: &Path) {
    if !package_root_is_complete(root) {
        panic!(
            "Windows App SDK packages are incomplete under {}",
            root.display()
        );
    }
}

fn fetch_packages(root: &Path) {
    fs::create_dir_all(root)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", root.display()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("fetch-windows-app-sdk.ps1");
    let status = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg("-PackageRoot")
        .arg(root)
        .status()
        .unwrap_or_else(|err| {
            panic!("failed to start PowerShell to fetch the Windows App SDK: {err}")
        });
    if !status.success() {
        panic!("Windows App SDK package download failed with exit status {status}");
    }
}
