# Changelog

## [0.1.0]

First release of rav1r. rav1r derives from [rav1e](https://github.com/xiph/rav1e) at commit `31435de9d76fddd38f6dcc31d4014574cebb2092`. The `upstream` remote and its git history hold the changes before that commit.

Removed:

- Binaries, the C API, the channel API, fuzz targets, benches, `tools/`, `cross/`, IVF and Y4M code, and wasm support.
- `generate_photon_noise_params`, `parse_grain_table`, `write_grain_table`, `DiffGenerator`, and the `TransferFunction` conversion methods, which `av1-grain` added to the prelude.

Changed:

- The default build is pure Rust and needs no NASM or C compiler. `asm` is off by default.
- `threading` selects `rayon`. Without it, the encoder runs on the calling thread.
- `v_frame`, `aligned-vec`, `av-scenechange`, and `av1-grain` are inlined or replaced in-tree. `NOTICE` records their licenses.
- Scene detection follows `RAV1E_CPU_TARGET`. `CPU_TARGET` has no effect.

Added:

- `tests/upstream_gate.rs` and `scripts/gate/`: a gate that compares packet bytes with upstream.
