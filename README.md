# nestix-native-winui

Experimental WinUI backend for [Nestix Native](https://github.com/wirelesseye/nestix-native).

## Requirements

- Windows with the MSVC Rust toolchain.
- PowerShell and network access the first time the pinned Windows App SDK
  package is downloaded.

Applications do not need the Windows App Runtime installed. The
`nestix-native-winui-build` helper creates an unpackaged, self-contained build
using the target architecture's Windows App Runtime MSIX.

## Application setup

Add the runtime crate and the application-owned build helper:

```toml
[dependencies]
nestix-native-winui = { git = "https://github.com/wirelesseye/nestix-native-winui.git" }

[build-dependencies]
nestix-native-winui-build = { git = "https://github.com/wirelesseye/nestix-native-winui.git" }
```

Call the helper from the application's `build.rs`:

```rust
fn main() {
    nestix_native_winui_build::configure();
}
```

Then build or run normally:

```powershell
cargo run
cargo build --release
```

The helper:

- downloads and verifies the pinned Windows App SDK NuGet packages when they
  are not already cached;
- extracts the target architecture's Windows App Runtime framework MSIX;
- stages its native runtime, metadata, resources, and locale files beside the
  executable;
- embeds the registration-free WinRT and DPI-awareness manifest into the final
  application executable.

Distribute the executable together with the DLL, PRI, WinMD, XAML, XBF,
resource, and locale files staged at the root of its Cargo profile directory.
Cargo's `build`, `deps`, and `incremental` bookkeeping directories are not
part of the application.

## Repository layout

- `nestix-native-winui` is the runtime library workspace member.
- `nestix-native-winui-build` is the application build helper and another
  workspace member.
- `examples/*` are workspace example applications.
- `tools/generate-bindings` is a maintainer-only tool outside the normal
  workspace build.

## Regenerating bindings and deployment metadata

Rust bindings are committed under `nestix-native-winui/src/bindings.rs`, so
consumer builds do not run `windows-bindgen` and do not require a source-local
`.packages` directory.

After changing the pinned Windows App SDK packages or binding filters:

```powershell
.\scripts\fetch-windows-app-sdk.ps1 -Force
.\scripts\generate-bindings.ps1
.\scripts\generate-self-contained-manifest.ps1
```

Package versions and SHA-256 checksums used by consumer builds are defined in
`nestix-native-winui-build`.
