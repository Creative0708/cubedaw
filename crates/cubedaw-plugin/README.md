# `cubedaw-plugin`
Data structures and whatnot for cubedaw plugins. Preprocesses WASM modules and JITs them into optimized patches.

# Terminology
- Patches: Instruments/postprocessing/whatever. Each track has an attached patch. Stolen from analog synth terminology.
- Plugin: WebAssembly module. Can contain multiple nodes.
- Node: Standalone module used in patches. Stolen from Blender terminology. Called a "module" in synth speak.

# Limitations
The plugin uses imported functions to mark special behavior that can be stitched in.
For example, "get 3rd input" would translate as
```wat
i32.const 2
call input
```
This is the best way of stitching that I could find (TODO: maybe talk about why this is the best for now?)
The problem with that is that stuff like:
```rs
let foo = if bar {
    input::<1>()
} else {
    input::<2>()
};
```
Could be compiled as
```wat
i32.const 1
i32.const 2
select       ; or something. you get the point
call input   ; uh oh! the parameter isn't deterministic anymore!! D:
```
Which is bad. Obviously cases like this could have special functionality built into the stitcher but there are too many cases to consider all of them.

So the plan is to stitch the simple cases (direct `i32.const` then `call`s & _maybe_ those select statements) then default to linking a fallback case.
