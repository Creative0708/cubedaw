# `cubedaw-plugin`
Data structures and whatnot for cubedaw plugins. Preprocesses WASM modules and JITs them into optimized patches.

# Terminology
- Patches: Instruments/postprocessing/whatever. Each track has an attached patch. Stolen from analog synth terminology.
- Plugin: WebAssembly module. Can contain multiple nodes.
- Node: Standalone module used in patches. Stolen from Blender terminology. Called a "module" in synth speak.
