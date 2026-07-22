# Agent Instructions

## Rust formatting

- Prefer `nestix-fmt` over invoking `cargo fmt` or `rustfmt` directly.
- From this repository's root, run `nestix-fmt --all` after changing Rust source files.
- Before finishing, run `nestix-fmt --check --all` to verify that Rust and embedded Nestix `layout!` code are formatted.
- If `nestix-fmt` is not installed or otherwise unavailable, fall back to `cargo fmt --all` and verify with `cargo fmt --all -- --check`.
