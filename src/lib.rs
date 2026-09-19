name: Build Native

on:
  push:
    paths:
      - 'src/**'
      - 'Cargo.toml'
      - '.github/workflows/build-native.yml'
  workflow_dispatch:

jobs:
  build-macos:
    runs-on: macos-13
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install cmake
        run: brew install cmake

      - name: Build
        run: cargo build --release

      - uses: actions/upload-artifact@v4
        with:
          name: libheic_transcoder-macos
          path: target/release/libheic_transcoder.dylib
