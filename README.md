# wgpu-matrix-simple

wgpuを使ったシンプルな行列乗算のサンプルプログラムです。

## 概要

このプログラムは、2つの正方行列をwgpuを使用してGPU上で乗算します。
結果はコンソールに出力されます。

## 実行方法

### ネイティブ実行

```bash
cargo run
```

### Web (Wasm) での実行

WebAssemblyとしてビルドし、Webサーバーでホストしてブラウザで実行します。

```bash
# Wasmビルドツール
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

# ビルド
cargo build --target wasm32-unknown-unknown

# Wasmに変換
wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/debug/wgpu-matrix-simple.wasm

# Webサーバーでホスト (例: Python)
python3 -m http.server --directory ./out
```
その後、ブラウザで `http://localhost:8000` にアクセスします。

## Raspberry Pi 4 での動作について

Raspberry Pi 4でこのプログラムを実行する場合、`wgpu`の制約により、`request_device`の`required_limits`を`wgpu::Limits::downlevel_defaults()`に設定する必要があります。

具体的には、`src/main.rs`の以下の部分を修正してください。

```diff
--- a/src/main.rs
+++ b/src/main.rs
@@ -66,7 +66,7 @@
         .request_device(&wgpu::DeviceDescriptor {
             label: None,
             required_features: wgpu::Features::empty(),
-            required_limits: wgpu::Limits::default(),
+            required_limits: wgpu::Limits::downlevel_defaults(),
             memory_hints: wgpu::MemoryHints::default(),
             trace: wgpu::Trace::Off,
         })
```