use anyhow::{ensure, Context as _};
use cargo_metadata as cm;
use itertools::Itertools as _;
use itertools_num::ItertoolsNum as _;
use lazy_static::lazy_static;
use maplit::hashmap;
use proc_macro2::{LineColumn, Span, TokenStream, TokenTree};
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs, iter,
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

    for src in fs::read_dir(ac_library_rs_manifest_path.with_file_name("src"))? {
        let src = src?.path();
        let code = fs::read_to_string(&src)?;
        let code = match src.file_stem().and_then(OsStr::to_str) {
            Some("lib") => modify_root_module(&code)?,
            Some(name) => modify_sub_module(name, &code)?,
            _ => unreachable!(),
        };
        let dst = out_dir.join(src.file_name().unwrap());
        fs::write(&dst, code)?;
        run_rustfmt(&dst)?;
    }
    Ok(())
}

fn modify_root_module(code: &str) -> anyhow::Result<String> {
    let syn::File { items, .. } = syn::parse_file(code)?;

    let mut pub_extern_crates = "".to_owned();
    let mut insertions = vec![];

    for item in items {
        if let Item::Mod(ItemMod { vis, ident, .. }) = &item {
            if matches!(vis, Visibility::Public(_)) {
                pub_extern_crates += &format!(
                    "pub extern crate __acl_{ident} as {ident};\n",
                    ident = ident,
                );
            }

            insertions.push((item.span().start(), "/*".to_owned()));
            insertions.push((item.span().end(), "*/".to_owned()));
        }
    }

    Ok(format!(
        "{}\npub use self::lib::*;\n\nmod lib {{\n{}}}\n",
        pub_extern_crates,
        indent(&replace_ranges(code, &[], &insertions)),
    ))
}

fn modify_sub_module(name: &str, code: &str) -> anyhow::Result<String> {
    fn visit_pub_visibilities(item: &Item, replacements: &mut Vec<(Span, String)>) {
        struct Visitor<'a> {
            replacements: &'a mut Vec<(Span, String)>,
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
                        self.replacements.push((i.span(), "pub".to_owned()));
                    }
                }
            }
        }

        Visitor { replacements }.visit_item(item)
    }

    lazy_static! {
        static ref DEPS: HashMap<&'static str, &'static [&'static str]> = hashmap!(
            "convolution" => &["internal_bit", "internal_math", "modint"][..],
            "lazysegtree" => &["internal_bit", "segtree"],
            "math" => &["internal_math"],
            "maxflow" => &["internal_type_traits", "internal_queue"],
            "mincostflow" => &["internal_type_traits"],
            "modint" => &["internal_math"],
            "scc" => &["internal_scc"],
            "segtree" => &["internal_bit", "internal_type_traits"],
            "twosat" => &["internal_scc"],
        );
    }

    let file = syn::parse_file(code)?;

    let mut replacements = vec![];

    for attr in &file.attrs {
        if let Ok(meta) = attr.parse_meta() {
            if meta.path().is_ident("doc") {
                replacements.push((attr.span(), "".to_owned()));
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
        code = indent(&replace_ranges(code, &replacements, &[])),
    ))
}

fn replace_ranges(
    code: &str,
    replacements: &[(Span, String)],
    insertions: &[(LineColumn, String)],
) -> String {
    // `proc-macro 1.0.10` is before this.
    // https://github.com/alexcrichton/proc-macro2/pull/229

    let from_line_columns = {
        let column_csum = iter::once(0)
            .chain(code.split('\n').map(|l| l.len() + 1))
            .cumsum()
            .collect::<Vec<usize>>();
        move |LineColumn { line, column }| column_csum[line - 1] + column
    };

    let mut code = code.as_bytes().to_owned();

    for (start, end, s) in replacements
        .iter()
        .map(|(span, s)| (span.start(), span.end(), s))
        .chain(insertions.iter().map(|(p, s)| (*p, *p, s)))
        .map(|(start, end, s)| (from_line_columns(start), from_line_columns(end), s))
        .sorted()
        .rev()
    {
        code = [&code[..start], s.as_ref(), &code[end..]].concat();
    }

    String::from_utf8(code)
        .expect("is something wrong? the version of `proc-macro2` should be `1.0.10`")
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
