# ac-library-rs-parted

[![CI](https://github.com/qryxip/ac-library-rs-parted/workflows/CI/badge.svg)](https://github.com/qryxip/ac-library-rs-parted/actions?workflow=CI)
[![dependency status](https://deps.rs/repo/github/qryxip/ac-library-rs-parted/status.svg)](https://deps.rs/repo/github/qryxip/ac-library-rs-parted)
[![Crates.io](https://img.shields.io/crates/v/ac-library-rs-parted.svg)](https://crates.io/crates/ac-library-rs-parted)
[![Crates.io](https://img.shields.io/crates/l/ac-library-rs-parted.svg)](https://crates.io/crates/ac-library-rs-parted)

Partitioned [ac-library-rs](https://github.com/rust-lang-ja/ac-library-rs).

## What is this?

ac-library-rs-parted is a collection of 17 crates that use modules of the real ac-library-rs.

```rust
// In each `$CARGO_MANIFEST_DIR/src/lib.rs`

::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/lib.rs"));
```

```rust
// In the `$OUT_DIR/lib.rs`

extern crate __acl_bar as bar;
extern crate __acl_baz as baz;

pub use self::foo::*;

mod foo {
    // The correspond `foo.rs` file that was modified as follows:
    //
    // - Replace `pub(crate)` to `pub`.
    // - Remove module doc, which cannot be directly included.
    // - Indent if it has no multi-line literals.
```

## License

Licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
