<div align="center">

# rav1r

<h3>AV1 encoder in Rust, derived from rav1e.</h3>

<p>
rav1r keeps the rav1e encoder library and drops the command-line tools, the C API, and the stale dependencies. The default build is pure Rust.
</p>

<p>
  <a href="#status"><strong>Status</strong></a>
  ·
  <a href="#usage"><strong>Usage</strong></a>
  ·
  <a href="#features"><strong>Features</strong></a>
  ·
  <a href="#what-differs-from-rav1e"><strong>What differs</strong></a>
  ·
  <a href="#origin"><strong>Origin</strong></a>
  ·
  <a href="CONTRIBUTING.md"><strong>Contributing</strong></a>
  ·
  <a href="#license-and-patents"><strong>License</strong></a>
</p>

<p>
  <a href="https://discord.gg/jBhFk9kHPg">
    <img src="https://img.shields.io/discord/1453357769720594543?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Join the Discord">
  </a>
</p>

<p>
  <a href="https://github.com/ml-rust/rav1r/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/ml-rust/rav1r/ci.yml?branch=master&label=ci" alt="CI status">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/github/license/ml-rust/rav1r" alt="License">
  </a>
  <a href="https://github.com/ml-rust/rav1r/stargazers">
    <img src="https://img.shields.io/github/stars/ml-rust/rav1r?style=social" alt="GitHub stars">
  </a>
</p>

</div>

rav1r is the AV1 encoder of the ml-rust media stack. [vidr](https://github.com/ml-rust/vidr) calls it as a library. Frames go in through `Context::send_frame` and AV1 packets come out of `Context::receive_packet`. Muxing lives in [containr](https://github.com/ml-rust/containr).

For the same input and config, rav1r encodes the same bytes as upstream rav1e. A test gate checks this on a fixed matrix of clips. The gate runs locally and not in CI. `asm` is off by default.

## Status

In development. rav1r has no release and no public repository yet. vidr reaches it through a local path patch.

Minimum Rust version: 1.85.

## Usage

This example shows the API. It encodes one blank frame:

```rust,no_run
use rav1r::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::default();
    let mut ctx: Context<u8> = cfg.new_context()?;
    ctx.send_frame(ctx.new_frame())?;
    ctx.flush();
    loop {
        match ctx.receive_packet() {
            Ok(packet) => println!("packet of {} bytes", packet.data.len()),
            Err(EncoderStatus::Encoded) => {}
            Err(EncoderStatus::LimitReached) => break,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}
```

[`examples/simple_encoding.rs`](examples/simple_encoding.rs) shows a longer run. [`doc/`](doc/README.md) holds the upstream design notes.

## Features

The default build needs only a Rust toolchain.

| Feature | What it adds | Default |
| ------- | ------------ | ------- |
| `threading` | `rayon` for tile and lookahead parallelism | Yes |
| `asm` | Hand-written assembly. Needs NASM 2.15 or newer on `x86_64`, or a C compiler on `aarch64` | No |
| `serialize` | `serde` support for the config and stats types | No |
| `unstable` | `Config::with_thread_pool` | No |
| `tracing` | Profiling spans through `tracing` | No |
| `dump_lookahead_data` | Lookahead debug data written to disk | No |
| `desync_finder` | Entropy coder desync debugging | No |
| `check_asm` | A comparison of each assembly routine with the Rust routine | No |
| `decode_test`, `decode_test_dav1d` | Decode with libaom or dav1d in tests | No |
| `quick_test` | Shorter encode-decode tests | No |

`asm` links the objects under `src/x86` and `src/arm`. Their headers name VideoLAN, the dav1d authors, Two Orioles, and Martin Storsjo, among others, and they stay in place.

## What differs from rav1e

### Removed

| Removed | Reason |
| ------- | ------ |
| `rav1e` and `rav1e-ch` binaries, `clap`, and the other CLI dependencies | rav1r is a library |
| C API (`capi`, `cbindgen.toml`) | No C caller exists |
| Channel API (`channel-api`) and `Config::with_parallel_gops` | Nothing uses it |
| Fuzz targets, benches, `tools/`, `cross/` | Each pulls in dependencies or toolchains |
| IVF and Y4M code (`ivf/`, `y4m`) | Containers belong to containr |
| wasm support (`wasm` feature, `wasm-bindgen`) | vidr does not target wasm |
| `built`, `simd_helpers`, `noop_proc_macro`, `arg_enum_proc_macro`, `maybe-rayon` | Stale. Small in-tree code replaces each |
| `v_frame`, `aligned-vec`, `av-scenechange`, `av1-grain` | Inlined. See the next table |

### Inlined

| Source | Now in | Effect on the API |
| ------ | ------ | ----------------- |
| `v_frame`, `aligned-vec` | `src/frame_buffer/` | None. Frame, plane, and pixel types keep their paths |
| `av-scenechange` | `src/scenechange/`, the subset the encoder calls | Scene detection uses rav1r's own SATD. Its SAD for fast detection is Rust only. `RAV1E_CPU_TARGET` applies, and `CPU_TARGET` does not. Output is unchanged |
| `av1-grain` | `src/api/config/grain.rs`, the film grain types | `generate_photon_noise_params`, `parse_grain_table`, `write_grain_table`, `DiffGenerator`, and the `TransferFunction` conversion methods are gone from the prelude |

### Changed

- `version` module: `version::hash` is gone. `version::long` and `version::full` return the crate version.
- Enums that derived `ArgEnum`: they keep `FromStr`, `Display`, and `variants()`. `variants()` returns a slice. `descriptions()` is gone.
- `asm`: off by default.
- `threading`: with it off, `rayon` is absent and the encoder runs on the calling thread.

### Kept

12-bit, 4:2:2, and 4:4:4 stay in the encoder, because they keep merges from upstream small.

## Origin

rav1r derives from [rav1e](https://github.com/xiph/rav1e) at commit `31435de9d76fddd38f6dcc31d4014574cebb2092` of its `master` branch. [NOTICE](NOTICE) holds the attribution.

The published rav1e 0.8.1 fails `cargo deny`, and vidr's `deny.toml` denies git sources. So vidr cannot depend on upstream. rav1r is a separate crate that vidr depends on.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for conventions, checks, and merging from upstream.

Questions and design discussion happen on [Discord](https://discord.gg/jBhFk9kHPg).

## License and patents

rav1r is licensed under the BSD 2-Clause License in [LICENSE](LICENSE). [PATENTS](PATENTS) holds the Alliance for Open Media Patent License 1.0.

The patent license sets conditions on every distribution:

- A source distribution carries `PATENTS` in its root directory.
- A binary distribution reproduces the AOM Patent License 1.0 in its documentation, legal notices, or other written materials. It reproduces the BSD license text there too.
- Section 1.3 of [PATENTS](PATENTS) ends the patent license for a licensee that sues over an Implementation, with two stated exceptions.

Original rav1e copyright notices stay in every file.
