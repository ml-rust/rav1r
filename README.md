<div align="center">

# rav1r

<h3>AV1 encoder in Rust, derived from rav1e.</h3>

<p>
rav1r keeps the rav1e encoder library and drops the command-line tools, the C API, and the stale dependencies. The default build is pure Rust.
</p>

<p>
  <a href="#origin"><strong>Origin</strong></a>
  ·
  <a href="#what-changed"><strong>What changed</strong></a>
  ·
  <a href="#features"><strong>Features</strong></a>
  ·
  <a href="#usage"><strong>Usage</strong></a>
  ·
  <a href="#taking-upstream-fixes"><strong>Upstream fixes</strong></a>
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

## Origin

rav1r derives from [rav1e](https://github.com/xiph/rav1e) at commit `31435de9d76fddd38f6dcc31d4014574cebb2092` of its `master` branch. [NOTICE](NOTICE) holds the attribution.

The published rav1e 0.8.1 fails `cargo deny`, and vidr's `deny.toml` denies git sources. So vidr cannot depend on upstream. This repository publishes its own crate.

## What changed

Removed, because vidr never calls it:

| Removed                                                                          | Reason                                          |
| -------------------------------------------------------------------------------- | ----------------------------------------------- |
| `rav1e` and `rav1e-ch` binaries, `clap`, and the other CLI dependencies          | rav1r is a library                              |
| C API (`capi`, `cbindgen.toml`)                                                  | No C caller exists                              |
| Channel API (`channel-api`) and `Config::with_parallel_gops`                     | Nothing uses it                                 |
| Fuzz targets, benches, `tools/`, `cross/`                                        | Each pulls dependencies or toolchains in        |
| IVF and Y4M code (`ivf/`, `y4m`)                                                 | Containers belong to containr                   |
| wasm support (`wasm` feature, `wasm-bindgen`)                                    | vidr does not target wasm                       |
| `built`, `simd_helpers`, `noop_proc_macro`, `arg_enum_proc_macro`, `maybe-rayon` | Stale. Small in-tree code replaces each of them |

Behaviour changes in the library:

- `version::hash` is gone. `version::long` and `version::full` return the crate version.
- The enums that derived `ArgEnum` keep `FromStr`, `Display`, and `variants()`. `variants()` returns a slice. `descriptions()` is gone.
- `asm` is off by default and it also controls `av-scenechange/asm`.
- With `threading` off, `rayon` is absent and the encoder runs on the calling thread.

Kept on purpose: 12-bit, 4:2:2 and 4:4:4 in the encoder. They keep merges from upstream small.

## Features

| Feature             | What it does                                                               |
| ------------------- | -------------------------------------------------------------------------- |
| `threading`         | Default. Uses `rayon` for tile and lookahead parallelism                   |
| `asm`               | Builds the hand-written assembly. Needs NASM 2.15 or newer on `x86_64`, and a C compiler on `aarch64` |
| `serialize`         | `serde` support for the config and stats types                             |
| `unstable`          | `Config::with_thread_pool`                                                 |
| `tracing`           | Emits profiling spans through `tracing`                                    |
| `dump_lookahead_data` | Writes lookahead debug data to disk                                      |
| `desync_finder`     | Entropy coder desync debugging                                             |
| `check_asm`         | Compares each assembly routine with the Rust routine                       |
| `decode_test`, `decode_test_dav1d` | Decode with libaom or dav1d in tests                        |
| `quick_test`        | Shortens the encode-decode tests                                           |

The default build needs only a Rust toolchain, version 1.85 or later. The `asm` feature links the objects under `src/x86` and `src/arm`. Their headers name VideoLAN, the dav1d authors, Two Orioles, and Martin Storsjo, among others, and they stay in place.

## Usage

```bash
cargo add rav1r
```

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

## Taking upstream fixes

The `upstream` remote points at `https://github.com/xiph/rav1e`.

1. Encode the reference clips on the current build and keep the packet bytes.
2. Run `git fetch upstream` and merge `upstream/master`.
3. Resolve each conflict at a removal site by keeping the removal. Keep `rustfmt.toml` and do not reformat files.
4. Encode the same clips with the same config and thread count. The packet bytes must match. A change that alters the bitstream gets named in the merge commit.
5. Run the checks in [AGENTS.md](AGENTS.md), with default features and with `asm`.

## License and patents

rav1r is licensed under the BSD 2-Clause License in [LICENSE](LICENSE). [PATENTS](PATENTS) holds the Alliance for Open Media Patent License 1.0.

The patent license sets conditions on every distribution:

- A source distribution carries `PATENTS` in its root directory.
- A binary distribution reproduces the AOM Patent License 1.0 in its documentation, legal notices, or other written materials. It reproduces the BSD license text there too.
- Section 1.3 of [PATENTS](PATENTS) ends the patent license for a licensee that sues over an Implementation, with two stated exceptions.

Original rav1e copyright notices stay in every file.
