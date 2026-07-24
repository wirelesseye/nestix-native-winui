# nestix-native-winui-build

Application-owned build support for `nestix-native-winui`.

Add this crate to the application's build dependencies and call `configure`
from the application—not the library—`build.rs`:

```rust
fn main() {
    nestix_native_winui_build::configure();
}
```

This embeds the self-contained Windows App SDK registration manifest into the
final executable and stages the target architecture's Windows App Runtime MSIX
payload beside it.
