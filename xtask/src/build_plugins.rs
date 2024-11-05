use anyhow::{Context, Result};
use xshell::{cmd, Shell};

pub fn run() -> Result<()> {
    let ctx = crate::Context::get();

    let sh = Shell::new()?;
    sh.change_dir(ctx.project_root.join("plugin"));

    let toolchain = toml::from_str::<toml::Value>(
        &sh.read_file("rust-toolchain.toml")
            .context("plugin/rust-toolchain.toml doesn't exist")?,
    )
    .context("failed to parse plugin/rust-toolchain.toml")?
    .get("toolchain")
    .and_then(|value| value.get("channel"))
    .and_then(|value| value.as_str())
    .context("invalid rust-toolchain.toml")?
    .to_owned();

    cmd!(
        sh,
        "cargo +{toolchain} build --release -p cubedaw-default-nodes"
    )
    .run()?;

    Ok(())
}
