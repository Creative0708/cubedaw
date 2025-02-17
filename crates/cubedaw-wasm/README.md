# cubedaw-wasm

Thin cross-platform wrapper around WebAssembly execution environments.
On native, uses [`wasmtime`](https://github.com/bytecodealliance/wasmtime).
On web, uses the [WebAssembly JS API](https://developer.mozilla.org/en-US/docs/WebAssembly/Using_the_JavaScript_API). (WIP)

The API surface of this crate is like `wasmtime`'s, but more restricted. Also, there's no documentation yet. :P
