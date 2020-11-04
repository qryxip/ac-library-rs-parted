use std::{env, path::PathBuf};
fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    xshell::write_file(out_dir.join("lib.rs"), ac_library_rs_parted_build::MAXFLOW)?;
    Ok(())
}
