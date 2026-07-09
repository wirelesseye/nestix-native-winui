# nestix-native-winui

Experimental WinUI backend for `nestix-native`.

## Prerequisites

- Windows with the MSVC Rust toolchain.
- PowerShell.
- Network access to NuGet for first-time setup.
- Windows App Runtime 1.8 installed for running framework-dependent apps.

The crate generates Rust bindings from Windows App SDK `.winmd` metadata with `windows-bindgen`. The metadata is downloaded into a local `.packages/` directory, which is intentionally ignored by Git.

## First-Time Setup

From this crate root:

```powershell
.\scripts\fetch-windows-app-sdk.ps1
```

This downloads and extracts the exact Windows App SDK NuGet packages expected by `build.rs`:

- `Microsoft.WindowsAppSDK` `1.8.260529003`
- `Microsoft.WindowsAppSDK.WinUI` `1.8.260528001`
- `Microsoft.WindowsAppSDK.Foundation` `1.8.260527000`
- `Microsoft.WindowsAppSDK.Base` `1.8.251216001`
- `Microsoft.WindowsAppSDK.InteractiveExperiences` `1.8.260525001`

To redownload and re-extract everything:

```powershell
.\scripts\fetch-windows-app-sdk.ps1 -Force
```

## Build

```powershell
cargo check
```

The build script:

- reads `.winmd` metadata from `.packages/`,
- generates WinUI bindings into Cargo's `OUT_DIR`,
- links `Microsoft.WindowsAppRuntime.Bootstrap.lib`,
- copies `Microsoft.WindowsAppRuntime.Bootstrap.dll` and `resources.pri` next to the built executable.

## Run Examples

```powershell
cargo run -p basic-winui
```

or:

```powershell
cargo run -p tabs
```

## Runtime Notes

The local `.packages/` folder is enough for build-time metadata and bootstrap import libraries, but it does not install the Windows App Runtime framework package globally.

If runtime bootstrap fails, install the Windows App Runtime 1.8 framework package or switch this crate to a self-contained deployment flow.

## Regenerating Bindings

Bindings are generated on every build from the metadata in `.packages/`. If the Windows App SDK package versions change, update both:

- `scripts/fetch-windows-app-sdk.ps1`
- the version constants in `build.rs`

Then run:

```powershell
.\scripts\fetch-windows-app-sdk.ps1 -Force
cargo check
```
