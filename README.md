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

extern crate __acl_foo as foo;
extern crate __acl_bar as bar;

pub use self::items::*;

mod items {
    // The correspond `.rs` file that was modified as follows:
    //
    // - Replace `pub(crate)` to `pub`.
    // - Remove doc comments.
}
```

## License

Licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
