# Physically Based Rendering GLTF Assets in Rust with WGPU

This renderer uses [wgpu](https://wgpu.rs/) to render a gltf asset using physically based rendering, supporting
both [wasm](https://webassembly.org/) and native.

```
# native
cargo run -r 

# web
trunk serve --open
```

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)