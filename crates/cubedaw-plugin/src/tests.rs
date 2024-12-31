
 // TODO: see cubedaw-wasm on reexporting wasmtime

#[test]
fn sanity_check_plugin_imports() {
    use crate::CubedawPluginImport;
    for import in CubedawPluginImport::ALL {
        assert_eq!(
            Some(import),
            CubedawPluginImport::new(import.name()),
            "import {import:?}'s name is not nameing"
        );
    }
}

/*
#[test]
fn test_basic_plugin() {
    let plugin = super::Plugin::new(
        &std::fs::read({
            // TODO not do this
            let mut path = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
            path.push(
                "/../../plugin/target/wasm32-unknown-unknown/release/cubedaw_test_plugin.wasm",
            );
            path
        })
        .unwrap(),
    )
    .unwrap();

    let mut module = stitch::ModuleStitch::with_imports(
        crate::ShimInfo::new(|mut ctx| {
            use crate::CubedawPluginImport;
            use wasm_encoder::Instruction;
            match ctx.import() {
                CubedawPluginImport::SampleRate => {
                    ctx.replace_only_current([Instruction::I32Const(44100)]);
                }
                CubedawPluginImport::Input => {
                    ctx.replace_only_current([]);
                    ctx.add_instruction_raw(Instruction::Call(0));
                }
                CubedawPluginImport::Output => {
                    ctx.replace_only_current([]);
                    ctx.add_instruction_raw(Instruction::Call(1));
                }
                CubedawPluginImport::Attribute => {
                    unimplemented!("the test plugin shouldn't contain any attribute calls");
                }
            }
        }),
        [CubedawPluginImport::Input, CubedawPluginImport::Output]
            .into_iter()
            .map(|import| (import.name(), import.ty())),
    );
    let mut func = stitch::FunctionStitch::new(FuncType::new([ValType::I32, ValType::I32], []));
    func.add_instruction_raw(&Instruction::LocalGet(0));
    func.add_instruction_raw(&Instruction::LocalGet(1));
    plugin
        .stitch_node(&resourcekey::literal!("test:test"), &mut func, &mut module)
        .unwrap();

    let func_idx = module.add_function(func);
    module.export_function("entrypoint", func_idx);
    module.export_memory("mem", 0);

    let bytes = module.finish();

    std::fs::write("/tmp/a.wasm", &bytes).unwrap();

    let config = WasmConfig::new().set_features(executing_wasm_features());

    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, &bytes).unwrap();

    let mut linker = Linker::new(&engine);

    // (input(0), input(1), output(0)
    type StoreData = Arc<(Mutex<[V128; 4]>, Mutex<[V128; 4]>, Mutex<[V128; 4]>)>;
    let store_data: StoreData = Arc::new((
        Mutex::new([V128::f32x4_splat(9.0); 4]),
        Mutex::new([V128::f32x4_splat(10.0); 4]),
        Mutex::new([V128::ZERO; 4]),
    ));
    linker
        .func_wrap(
            "host",
            CubedawPluginImport::Input.name(),
            |caller: cubedaw_wasm::wasmtime::Caller<'_, StoreData>, input_idx: u32| {
                let data = caller.data();

                let arr = match input_idx {
                    0 => *data.0.lock().unwrap(),
                    1 => *data.1.lock().unwrap(),
                    _ => [V128::ZERO; 4],
                };
                let [a, b, c, d] = arr;
                (
                    OtherV128::from(a),
                    OtherV128::from(b),
                    OtherV128::from(c),
                    OtherV128::from(d),
                )
            },
        )
        .unwrap();
    linker
        .func_wrap(
            "host",
            CubedawPluginImport::Output.name(),
            |caller: cubedaw_wasm::wasmtime::Caller<'_, StoreData>,
             a: OtherV128,
             b: OtherV128,
             c: OtherV128,
             d: OtherV128,
             output_idx: u32| {
                let arr: [V128; 4] = [a.into(), b.into(), c.into(), d.into()];

                let data = caller.data();

                match output_idx {
                    0 => *data.2.lock().unwrap() = arr,
                    1 => drop(Box::new(())), // to shut clippy up
                    _ => (),
                };
            },
        )
        .unwrap();

    let mut store = Store::new(&engine, store_data.clone());

    let instance = linker.instantiate(&mut store, &module).unwrap();

    let memory = instance
        .get_memory(&mut store, &module.get_export("mem").unwrap())
        .unwrap();

    let args_offset = memory.size(&store);
    let state_offset = args_offset + 64;
    let num_pages = 65536 >> memory.page_size_log2(&store);

    memory.grow(&mut store, num_pages).unwrap();

    let mut run = |args: u8, state: &mut [V128; 4]| {
        // TestPluginArgs
        memory.write(&mut store, args_offset, &[args]).unwrap();
        // TestPluginState
        memory
            .write(&mut store, state_offset, bytemuck::must_cast_slice(state))
            .unwrap();

        let func = instance
            .get_func(&mut store, &module.get_export("entrypoint").unwrap())
            .unwrap();

        func.call(
            &mut store,
            &[
                Value::I32(args_offset as i32),
                Value::I32(state_offset as i32),
            ],
            &mut [],
        )
        .unwrap();

        memory
            .read(&store, state_offset, bytemuck::must_cast_slice_mut(state))
            .unwrap();
    };

    let mut v128s: [V128; 4] = [V128::ZERO; 4];

    run(0u8, &mut v128s);
    assert_eq!(v128s, [V128::f32x4_splat(19.0); 4]);

    run(1u8, &mut v128s);
    assert_eq!(v128s, [V128::f32x4_splat(90.0); 4]);

    run(2u8, &mut v128s);
    assert_eq!(v128s, [V128::f32x4_splat(44100.0); 4]);
}
*/
