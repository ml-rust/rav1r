# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-06

### Changed

- `README`, crate description: state what the crate does.

## [0.1.0] - 2026-11-05

First release. rav1r derives from [rav1e](https://github.com/xiph/rav1e) at commit `31435de9d76fddd38f6dcc31d4014574cebb2092`. rav1e's git history holds the changes before that commit.

### Removed

- `Config::with_parallel_gops`: gone with the channel API. Nothing replaces it.
- `generate_photon_noise_params`, `parse_grain_table`, `write_grain_table`, `DiffGenerator`, and the `TransferFunction` conversion methods: `av1-grain` added them to the prelude, and they are gone.
- `version::hash` and `descriptions()`: gone.
- Binaries, the C API, the channel API, fuzz targets, benches, `tools/`, `cross/`, IVF and Y4M code, and wasm support: gone.

### Changed

- `asm`: off by default. The default build is pure Rust and needs no NASM or C compiler.
- `threading`: selects `rayon`. Without it, the encoder runs on the calling thread.
- `variants()`: returns a slice. `version::long` and `version::full` return the crate version.
- `v_frame`, `aligned-vec`, `av-scenechange`, and `av1-grain`: inlined or replaced in-tree. `NOTICE` records their licenses.
- Scene detection: follows `RAV1E_CPU_TARGET`. `CPU_TARGET` has no effect.
