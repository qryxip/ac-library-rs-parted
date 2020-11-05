use anyhow::{ensure, Context as _};
use cargo_metadata as cm;
use if_chain::if_chain;
use itertools::Itertools as _;
use maplit::{btreemap, hashmap};
use once_cell::sync::Lazy;
use proc_macro2::{LineColumn, TokenStream, TokenTree};
use std::{
    collections::{BTreeMap, HashMap},
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};
use syn::{spanned::Spanned, visit::Visit, Item, ItemMod, VisRestricted, Visibility};

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let metadata = cm::MetadataCommand::new()
        .manifest_path(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .exec()?;

    let ac_library_rs_manifest_path = (|| {
        let cm::Resolve { nodes, root, .. } = metadata.resolve.as_ref()?;
        let root = root.as_ref()?;
        let cm::NodeDep { pkg, .. } = nodes
            .iter()
            .find(|cm::Node { id, .. }| id == root)?
            .deps
            .iter()
            .filter(|cm::NodeDep { pkg, dep_kinds, .. }| {
                metadata[pkg].name == "ac-library-rs"
                    && matches!(
                        **dep_kinds,
                        [cm::DepKindInfo {
                            kind: cm::DependencyKind::Build,
                            ..
                        }]
                    )
            })
            .exactly_one()
            .ok()?;
        Some(&metadata[pkg].manifest_path)
    })()
    .with_context(|| "could not find the `ac-library-rs`")?;

    for src in xshell::read_dir(ac_library_rs_manifest_path.with_file_name("src"))? {
        let code = xshell::read_file(&src)?;
        let code = match src.file_stem().and_then(OsStr::to_str) {
            Some("lib") => modify_root_module(&code)?,
            Some(name) => modify_sub_module(name, &code)?,
            _ => unreachable!(),
        };
        let dst = out_dir.join(src.file_name().unwrap());
        xshell::write_file(&dst, code)?;
        run_rustfmt(&dst)?;
    }
    Ok(())
}

fn modify_root_module(code: &str) -> anyhow::Result<String> {
    let syn::File { items, .. } = syn::parse_file(code)?;

    let mut pub_extern_crates = "".to_owned();
    let mut replacements = btreemap!();

    for item in items {
        if let Item::Mod(ItemMod { vis, ident, .. }) = &item {
            if matches!(vis, Visibility::Public(_)) {
                pub_extern_crates += &format!(
                    "pub extern crate __acl_{ident} as {ident};\n",
                    ident = ident,
                );
            }

            let pos = item.span().start();
            replacements.insert((pos, pos), "/*".to_owned());
            let pos = item.span().end();
            replacements.insert((pos, pos), "*/".to_owned());
        }
    }

    Ok(format!(
        "{}\npub use self::lib::*;\n\nmod lib {{\n{}}}\n",
        pub_extern_crates,
        indent(&replace_ranges(code, replacements)),
    ))
}

fn modify_sub_module(name: &str, code: &str) -> anyhow::Result<String> {
    fn visit_pub_visibilities(
        item: &Item,
        replacements: &mut BTreeMap<(LineColumn, LineColumn), String>,
    ) {
        struct Visitor<'a> {
            replacements: &'a mut BTreeMap<(LineColumn, LineColumn), String>,
        }

        impl Visit<'_> for Visitor<'_> {
            fn visit_visibility(&mut self, i: &Visibility) {
                if let Visibility::Restricted(VisRestricted {
                    in_token: None,
                    path,
                    ..
                }) = i
                {
                    if path.is_ident("crate") {
                        self.replacements
                            .insert((i.span().start(), i.span().end()), "pub".to_owned());
                    }
                }
            }
        }

        Visitor { replacements }.visit_item(item)
    }

    static DEPS: Lazy<HashMap<&str, &[&str]>> = Lazy::new(|| {
        hashmap!(
            "convolution" => &["internal_bit", "internal_math", "modint"][..],
            "lazysegtree" => &["internal_bit", "segtree"],
            "math" => &["internal_math"],
            "maxflow" => &["internal_type_traits", "internal_queue"],
            "mincostflow" => &["internal_type_traits"],
            "modint" => &["internal_math"],
            "scc" => &["internal_scc"],
            "segtree" => &["internal_bit", "internal_type_traits"],
            "twosat" => &["internal_scc"],
        )
    });

    let file = syn::parse_file(code)?;

    let mut replacements = btreemap!();

    for attr in &file.attrs {
        if let Ok(meta) = attr.parse_meta() {
            if meta.path().is_ident("doc") {
                replacements.insert((attr.span().start(), attr.span().end()), "".to_owned());
            }
        }
    }

    for item in &file.items {
        visit_pub_visibilities(&item, &mut replacements);
    }

    Ok(format!(
        "{extern_crates}pub use self::{name}::*;\n\nmod {name} {{\n{code}}}\n",
        extern_crates = DEPS
            .get(name)
            .unwrap_or(&&[][..])
            .iter()
            .map(|dep| format!("extern crate __acl_{dep} as {dep};\n", dep = dep))
            .format(""),
        name = name,
        code = indent(&replace_ranges(code, replacements)),
    ))
}

fn replace_ranges(code: &str, replacements: BTreeMap<(LineColumn, LineColumn), String>) -> String {
    let replacements = replacements.into_iter().collect::<Vec<_>>();
    let mut replacements = &*replacements;
    let mut skip_until = None;
    let mut ret = "".to_owned();
    let mut lines = code.trim_end().split('\n').enumerate().peekable();
    while let Some((i, s)) = lines.next() {
        for (j, c) in s.chars().enumerate() {
            if_chain! {
                if let Some(((start, end), replacement)) = replacements.get(0);
                if (i, j) == (start.line - 1, start.column);
                then {
                    ret += replacement;
                    if start == end {
                        ret.push(c);
                    } else {
                        skip_until = Some(*end);
                    }
                    replacements = &replacements[1..];
                } else {
                    if !matches!(skip_until, Some(LineColumn { line, column }) if (i, j) < (line - 1, column)) {
                        ret.push(c);
                        skip_until = None;
                    }
                }
            }
        }
        while let Some(((start, end), replacement)) = replacements.get(0) {
            if i == start.line - 1 {
                ret += replacement;
                if start < end {
                    skip_until = Some(*end);
                }
                replacements = &replacements[1..];
            } else {
                break;
            }
        }
        if lines.peek().is_some() || code.ends_with('\n') {
            ret += "\n";
        }
    }

    debug_assert!(syn::parse_file(&code).is_ok());
    ret
}

fn indent(code: &str) -> String {
    let is_safe = !code
        .parse::<TokenStream>()
        .into_iter()
        .flat_map(IntoIterator::into_iter)
        .any(|tt| {
            matches!(
                tt, TokenTree::Literal(lit)
                if lit.span().start().line != lit.span().end().line
            )
        });

    if is_safe {
        code.lines()
            .map(|line| match line {
                "" => "\n".to_owned(),
                line => format!("    {}\n", line),
            })
            .join("")
    } else {
        code.to_owned()
    }
}

fn run_rustfmt(path: &Path) -> anyhow::Result<()> {
    let rustfmt_exe = Path::new(env!("CARGO"))
        .with_file_name("rustfmt")
        .with_extension(env::consts::EXE_EXTENSION);

    ensure!(
        rustfmt_exe.exists(),
        "{} does not exist",
        rustfmt_exe.display()
    );

    let status = Command::new(&rustfmt_exe)
        .args(&["--edition", "2018"])
        .arg(path)
        .status()?;

    ensure!(status.success(), "`rustfmt` failed");
    Ok(())
}
