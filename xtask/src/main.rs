use anyhow::{anyhow, Context as _};
use cargo_metadata as cm;
use duct::cmd;
use itertools::Itertools as _;
use proc_macro2::{LineColumn, Span};
use quote::quote;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{self, PathBuf},
};
use structopt::StructOpt;
use syn::{spanned::Spanned as _, visit::Visit, Item, Lit, Meta, MetaNameValue, Visibility};

#[derive(StructOpt)]
struct Opt {}

fn main() -> anyhow::Result<()> {
    Opt::from_args();

    let metadata_for_ac_library_rs_parted = cargo_metadata("./Cargo.toml")?;
    let metadata_for_xtask = cargo_metadata("./xtask/Cargo.toml")?;

    let ac_library_rs_parted = metadata_for_ac_library_rs_parted.resolve_root()?;
    let xtask = metadata_for_xtask.resolve_root()?;
    let ac_library_rs =
        metadata_for_xtask.find_lib_by_extern_crate_name(&xtask.id, "_ac_library_rs")?;

    for module_name in &[
        "convolution",
        "dsu",
        "fenwicktree",
        "internal_bit",
        "internal_math",
        "internal_queue",
        "internal_scc",
        "internal_type_traits",
        "lazysegtree",
        "math",
        "maxflow",
        "mincostflow",
        "modint",
        "scc",
        "segtree",
        "string",
        "twosat",
    ] {
        let ac_library_rs_parted_x = metadata_for_ac_library_rs_parted.workspace_member(
            &format!("ac-library-rs-parted-{}", module_name.replace('_', "-")),
        )?;

        let extern_crate_names = metadata_for_ac_library_rs_parted
            .extern_crate_names_for_normal_dependencies(&ac_library_rs_parted_x.id);

        let module_file_path = ac_library_rs
            .src_path
            .with_file_name(module_name)
            .with_extension("rs");

        let (code, doc) = take_crate_level_doc(&read_file(module_file_path)?)?;
        let code = replace_vis_stricts(&code)?;
        let code = format!(
            "// This code was expanded by `xtask`.\n\
             \n\
             {doc}\n\
             \n\
             {extern_crates}\n\
             \n\
             pub use self::{module_name}::*;\n\
             \n\
             mod {module_name} {{\n\
             {code}
             }}\n",
            doc = quote!(#(#![doc = #doc])*),
            extern_crates = extern_crate_names
                .into_iter()
                .map(|from| {
                    let to = from.trim_start_matches("__acl_");
                    format!("extern crate {} as {};", from, to)
                })
                .join("\n"),
            module_name = module_name,
            // consider not to contain mult-line literals.
            code = code
                .lines()
                .map(|line| match line {
                    "" => "\n".to_owned(),
                    line => format!("    {}\n", line),
                })
                .join(""),
        );

        let cm::Target { src_path, .. } = ac_library_rs_parted_x.lib_target()?;
        write_file(src_path, code)?;
        rustfmt(src_path)?;
        eprintln!("    Modified {}", src_path.display());
    }

    let cm::Target { src_path, .. } = ac_library_rs;
    let code = modify_top(&read_file(src_path)?)?;
    let cm::Target { src_path, .. } = ac_library_rs_parted.lib_target()?;
    write_file(src_path, code)?;
    rustfmt(src_path)?;
    eprintln!("    Modified {}", src_path.display());

    Ok(())
}

fn cargo_metadata(manifest_path: impl AsRef<path::Path>) -> anyhow::Result<cm::Metadata> {
    cm::MetadataCommand::new()
        .manifest_path(manifest_path.as_ref())
        .exec()
        .map_err(|err| match err {
            cm::Error::CargoMetadata { stderr } => {
                anyhow!("{}", stderr.trim_start_matches("error: ").trim_end())
            }
            err => anyhow::Error::msg(err),
        })
}

fn take_crate_level_doc(code: &str) -> syn::Result<(String, Vec<String>)> {
    let syn::File { attrs, .. } = syn::parse_file(code)?;

    let (replace_with, doc) = attrs
        .iter()
        .flat_map(|attr| match attr.parse_meta().ok()? {
            Meta::NameValue(MetaNameValue {
                path,
                lit: Lit::Str(lit_str),
                ..
            }) if path.is_ident("doc") => Some(((attr.span(), "".to_owned()), lit_str.value())),
            _ => None,
        })
        .unzip::<_, _, Vec<_>, _>();

    Ok((replace_ranges(code, &replace_with), doc))
}

fn replace_vis_stricts(code: &str) -> syn::Result<String> {
    let mut replace_with = vec![];
    Visitor {
        replace_with: &mut replace_with,
    }
    .visit_file(&syn::parse_file(code)?);
    return Ok(replace_ranges(code, &replace_with));

    struct Visitor<'a> {
        replace_with: &'a mut Vec<(Span, String)>,
    };

    impl Visit<'_> for Visitor<'_> {
        fn visit_visibility(&mut self, i: &'_ Visibility) {
            if let Visibility::Restricted(_) = i {
                self.replace_with.push((i.span(), "pub".to_owned()));
            }
        }
    }
}

fn modify_top(code: &str) -> syn::Result<String> {
    let syn::File { items, .. } = syn::parse_file(code)?;

    let replace_with = items
        .iter()
        .flat_map(|item| match item {
            Item::Mod(item_mod) => Some(item_mod),
            _ => None,
        })
        .map(|item_mod| {
            let span = item_mod.span();
            let replace_with = if let Visibility::Public(_) = item_mod.vis {
                format!(
                    "pub extern crate __acl_{ident} as {ident};",
                    ident = item_mod.ident
                )
            } else {
                "".to_owned()
            };
            (span, replace_with)
        })
        .collect::<Vec<_>>();

    Ok(replace_ranges(code, &replace_with))
}

fn replace_ranges(code: &str, with: &[(Span, String)]) -> String {
    let to_range = {
        let lines = code.split('\n').collect::<Vec<_>>();
        move |span: Span| -> _ {
            let from_pos = |loc: LineColumn| {
                lines[..loc.line - 1]
                    .iter()
                    .map(|s| s.len() + 1)
                    .sum::<usize>()
                    + lines[loc.line - 1]
                        .char_indices()
                        .nth(loc.column)
                        .map(|(i, _)| i)
                        .unwrap_or_else(|| lines[loc.line - 1].len())
            };
            from_pos(span.start())..from_pos(span.end())
        }
    };

    let mut code = code.to_owned();
    for (span, with) in with.iter().rev() {
        code.replace_range(to_range(*span), with);
    }
    code
}

fn rustfmt(path: &path::Path) -> anyhow::Result<()> {
    let rustfmt_exe = PathBuf::from(
        env::var_os("CARGO").with_context(|| "missing `$CARGO` environment variable")?,
    )
    .with_file_name("rustfmt")
    .with_extension(env::consts::EXE_EXTENSION);

    cmd!(rustfmt_exe, "--edition", "2018", &path).run()?;
    Ok(())
}

fn read_file(path: impl AsRef<path::Path>) -> anyhow::Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path).with_context(|| format!("could not read `{}`", path.display()))
}

fn write_file(path: impl AsRef<path::Path>, contents: impl AsRef<[u8]>) -> anyhow::Result<()> {
    let path = path.as_ref();
    fs::write(path, contents).with_context(|| format!("could not write `{}`", path.display()))
}

trait MetadataExt {
    fn resolve(&self) -> &cm::Resolve;
    fn resolve_root(&self) -> anyhow::Result<&cm::Package>;
    fn node(&self, package_id: &cm::PackageId) -> &cm::Node;
    fn workspace_member(&self, name: &str) -> anyhow::Result<&cm::Package>;
    fn find_lib_by_extern_crate_name(
        &self,
        from: &cm::PackageId,
        extern_crate_name: &str,
    ) -> anyhow::Result<&cm::Target>;
    fn extern_crate_names_for_normal_dependencies(&self, from: &cm::PackageId) -> BTreeSet<&str>;
}

impl MetadataExt for cm::Metadata {
    fn resolve(&self) -> &cm::Resolve {
        self.resolve.as_ref().expect("should be present")
    }

    fn resolve_root(&self) -> anyhow::Result<&cm::Package> {
        let resolve_root = self
            .resolve()
            .root
            .as_ref()
            .with_context(|| "this is a virtual workspace")?;
        Ok(&self[resolve_root])
    }

    fn node(&self, package_id: &cm::PackageId) -> &cm::Node {
        self.resolve()
            .nodes
            .iter()
            .find(|cm::Node { id, .. }| id == package_id)
            .unwrap_or_else(|| panic!("`{}` not found", package_id))
    }

    fn workspace_member(&self, name: &str) -> anyhow::Result<&cm::Package> {
        self.packages
            .iter()
            .find(|p| self.workspace_members.contains(&p.id) && p.name == name)
            .with_context(|| format!("`{}` not in the workspace", name))
    }

    fn find_lib_by_extern_crate_name(
        &self,
        from: &cm::PackageId,
        extern_crate_name: &str,
    ) -> anyhow::Result<&cm::Target> {
        let cm::NodeDep { pkg, .. } = self
            .resolve()
            .nodes
            .iter()
            .find(|cm::Node { id, .. }| id == from)
            .unwrap_or_else(|| panic!("`{}` not found", from))
            .deps
            .iter()
            .find(|cm::NodeDep { name, .. }| name == extern_crate_name)
            .with_context(|| {
                format!("could not find `{}` ===`{}`==> ?", from, extern_crate_name)
            })?;

        self[pkg]
            .targets
            .iter()
            .find(|cm::Target { kind, .. }| *kind == ["lib".to_owned()])
            .with_context(|| format!("`{}` does not contain `lib` target", pkg))
    }

    fn extern_crate_names_for_normal_dependencies(&self, from: &cm::PackageId) -> BTreeSet<&str> {
        self.node(from)
            .deps
            .iter()
            .filter(|cm::NodeDep { dep_kinds, .. }| {
                dep_kinds
                    .iter()
                    .any(|cm::DepKindInfo { kind, .. }| *kind == cm::DependencyKind::Normal)
            })
            .map(|cm::NodeDep { name, .. }| &**name)
            .collect()
    }
}

trait PackageExt {
    fn lib_target(&self) -> anyhow::Result<&cm::Target>;
}

impl PackageExt for cm::Package {
    fn lib_target(&self) -> anyhow::Result<&cm::Target> {
        self.targets
            .iter()
            .find(|cm::Target { kind, .. }| *kind == ["lib".to_owned()])
            .with_context(|| format!("could not find `lib` target in {}", self.id))
    }
}
