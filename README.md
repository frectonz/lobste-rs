# lobste-rs

A CLI client for [lobste.rs](https://lobste.rs) written in Rust.

## Demo

[![asciicast](https://asciinema.org/a/586643.svg)](https://asciinema.org/a/586643)

## Installation

```bash
cargo install --git https://github.com/frectonz/lobste-rs
```

If you don't have Rust installed, you can use the pre-built binaries from the releases page.

Download the binary for your platform.

- If you are on **macOS** download the file that ends with `x86_64-apple-darwin.zip`
- If you are on **windows** download the file that ends with `x86_64-pc-windows-gnu.zip`
- If you are on **linux** download one of the files that end with the following:
  - `x86_64-unknown-linux-musl.tar.gz`
  - `x86_64-unknown-linux-musl.tar.xz`
  - `x86_64-unknown-linux-musl.tar.zst`

For linux users there is no difference between the three files. You can use any of them. The difference is the compression algorithm used. So you can choose the one with the smallest size.

If you are on linux or macOS, you can use the following command to extract the binary from the archive:

```bash
tar xvf <downloaded_file>
```
