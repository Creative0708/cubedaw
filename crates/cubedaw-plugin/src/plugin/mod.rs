mod function;
mod instructions;
mod misc;
mod module;
mod stitch;

// TODO decide whether this is fine
pub type CubedawPlugin = module::PreparedModule;

#[cfg(test)]
mod tests {
    use super::stitch;

    #[test]
    fn test_basic() {
        let mut plugin = super::CubedawPlugin::new(
            &std::fs::read({
                let mut path = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
                path.push(
                    "/../../plugin/target/wasm32-unknown-unknown/debug/deps/cubedaw_default_plugins.wasm",
                );
                println!("plugin path: {path:?}");
                path
            })
            .unwrap(),
        )
        .unwrap();

        let mut module = stitch::ModuleStitch::new();
        let mut func = stitch::FunctionStitch::new();
        plugin.stitch(&mut func, &mut module);

        dbg!(func);
        // dbg!(module);

        panic!();
    }
}
