# Web

## Compilation

First install wasm-bindgen and add the wasm32-unknown-unknown target with rustup. Then run:

```bash
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-name hierarchical-wfc \
  --out-dir web \
  --target web target/wasm32-unknown-unknown/release/hierarchical-wfc.wasm
```

You can also modify [index.html](./index.html) to suit your needs.
