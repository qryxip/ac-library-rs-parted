pub use anyhow;

#[macro_export]
macro_rules! main {
    ($content:expr $(,)?) => {
        fn main() -> $crate::anyhow::Result<()> {
            let out_dir = ::std::path::PathBuf::from(::std::env::var_os("OUT_DIR").unwrap());
            ::std::fs::write(out_dir.join("lib.rs"), $content)?;
            Ok(())
        }
    };
}

pub static CONVOLUTION: &str = include_str!(concat!(env!("OUT_DIR"), "/convolution.rs"));
pub static DSU: &str = include_str!(concat!(env!("OUT_DIR"), "/dsu.rs"));
pub static FENWICKTREE: &str = include_str!(concat!(env!("OUT_DIR"), "/fenwicktree.rs"));
pub static INTERNAL_BIT: &str = include_str!(concat!(env!("OUT_DIR"), "/internal_bit.rs"));
pub static LAZYSEGTREE: &str = include_str!(concat!(env!("OUT_DIR"), "/lazysegtree.rs"));
pub static LIB: &str = include_str!(concat!(env!("OUT_DIR"), "/lib.rs"));
pub static MATH: &str = include_str!(concat!(env!("OUT_DIR"), "/math.rs"));
pub static MAXFLOW: &str = include_str!(concat!(env!("OUT_DIR"), "/maxflow.rs"));
pub static MINCOSTFLOW: &str = include_str!(concat!(env!("OUT_DIR"), "/mincostflow.rs"));
pub static MODINT: &str = include_str!(concat!(env!("OUT_DIR"), "/modint.rs"));
pub static SCC: &str = include_str!(concat!(env!("OUT_DIR"), "/scc.rs"));
pub static SEGTREE: &str = include_str!(concat!(env!("OUT_DIR"), "/segtree.rs"));
pub static STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/string.rs"));
pub static INTERNAL_MATH: &str = include_str!(concat!(env!("OUT_DIR"), "/internal_math.rs"));
pub static INTERNAL_QUEUE: &str = include_str!(concat!(env!("OUT_DIR"), "/internal_queue.rs"));
pub static INTERNAL_SCC: &str = include_str!(concat!(env!("OUT_DIR"), "/internal_scc.rs"));
pub static INTERNAL_TYPE_TRAITS: &str =
    include_str!(concat!(env!("OUT_DIR"), "/internal_type_traits.rs"));
pub static TWOSAT: &str = include_str!(concat!(env!("OUT_DIR"), "/twosat.rs"));
