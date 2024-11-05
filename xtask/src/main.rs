use std::{
    env,
    path::{Path, PathBuf},
};

mod build_plugins;

fn main() -> anyhow::Result<()> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("build_plugins") => build_plugins::run()?,
        Some(other) => {
            eprintln!("unknown xtask: {other}");
        }
        None => {
            eprintln!("TODO task list. go check out {} in the mean time", file!());
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Context {
    pub project_root: PathBuf,
}

impl Context {
    pub fn get() -> &'static Self {
        use std::sync::LazyLock;
        static SINGLETON: LazyLock<Context> = LazyLock::new(|| {
            let project_root = Path::new(
                &env::var("CARGO_MANIFEST_DIR")
                    .expect("CARGO_MANIFEST_DIR should be set by cargo when running"),
            )
            .ancestors()
            .nth(1)
            .unwrap()
            .to_path_buf();

            Context { project_root }
        });

        &SINGLETON
    }
}
