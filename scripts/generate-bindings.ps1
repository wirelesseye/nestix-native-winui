$ErrorActionPreference = "Stop"

cargo run --offline --manifest-path tools/generate-bindings/Cargo.toml
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
