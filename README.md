# ac-library-rs-parted

[![CI](https://github.com/qryxip/ac-library-rs-parted/workflows/CI/badge.svg)](https://github.com/qryxip/ac-library-rs-parted/actions?workflow=CI)
[![Rust 2018 1.41.1+](https://img.shields.io/badge/rust%202018-1.41.1+-lightgray.svg)](https://www.rust-lang.org)
![Crates.io](https://img.shields.io/badge/crates.io-not%20yet-inactive)
![License](https://img.shields.io/badge/license-CC0--1.0-informational)

Partitioned [ac-library-rs](https://github.com/rust-lang-ja/ac-library-rs).

## What is this?

ac-library-rs-parted is a collection of 17 crates that use modules from the real ac-library-rs.

```rust
//! Module-level document from the original ac-library-rs

extern crate __acl_bar as bar;
extern crate __acl_baz as baz;

pub use self::foo::*;

mod foo {
    // The correspond `foo.rs` file that was modified as follows:
    //
    // - Replace `pub(crate)` to `pub`.
    // - Indent if it has no multi-line literals.
}
```

## How to update this repository

```console
❯ cargo update --manifest-path ./xtask/Cargo.toml && cargo xtask
```

## License

Licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
