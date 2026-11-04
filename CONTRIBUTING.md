# Contributing to rav1r

rav1r is the AV1 encoder of the ml-rust media stack. It derives from rav1e and stays mergeable with it. [AGENTS.md](AGENTS.md) holds the placement rules and invariants.

## Before you start

Open an issue before you start anything non-trivial. Discuss design on [Discord](https://discord.gg/jBhFk9kHPg).

## Toolchain

- Stable Rust, version 1.85 or later.
- `cargo-deny` and `shellcheck` for the checks.
- NASM 2.15 or newer on `x86_64`, or a C compiler on `aarch64`. Only `asm` and the gate's reference build need them.

## Conventions

- Keep each edit minimal at its site. Change the lines that need it and no others.
- Never run `cargo fmt` across the tree to reformat it. Run `rustfmt` only on a file you rewrote substantially or created. `rustfmt.toml` sets 2 spaces and width 79.
- Keep `LICENSE`, `PATENTS`, and every per-file header. Keep `NOTICE` current.
- Keep the `rav1e_` assembly symbols. A merge from upstream needs them.
- Make every optional capability a Cargo feature. `asm` stays out of `default`.
- Add a dependency only if it has a release or commit in the last year.
- Add no binary, C API, fuzz target, bench harness, or container code.

## Checks

```sh
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --no-default-features --features threading -- -D warnings
cargo clippy --all-targets --no-default-features --features asm -- -D warnings
cargo clippy --all-targets --no-default-features --features serialize -- -D warnings
cargo test --no-fail-fast
cargo test --no-default-features --no-fail-fast
cargo fmt --all --check
cargo deny --exclude-dev check
shellcheck scripts/ci/*.sh scripts/gate/*.sh
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features threading,serialize
```

CI also checks the library on the MSRV in `Cargo.toml`.

Encode-decode tests need libaom and libdav1d:

```sh
sudo apt install libaom-dev libdav1d-dev
cargo test --release --features decode_test --lib -- test_encode_decode
cargo test --release --features decode_test_dav1d --lib -- test_encode_decode
```

## Decode-back tests

`tests/decode_back.rs` proves that independent decoders reproduce the encoder's own reconstruction. Each case runs these steps:

1. Encodes a seeded clip.
2. Writes an IVF file under `CARGO_TARGET_TMPDIR`.
3. Decodes it with a command line tool.
4. Compares every plane byte for byte with `Packet.rec`.

The matrix covers:

- speeds 0 to 10
- 8 and 10 bit
- odd sizes
- tiles
- worker counts
- key frame intervals
- low latency
- quantizer mode and bitrate mode

| Test | Decoder |
| --- | --- |
| `a10_dav1d` | `dav1d` |
| `a11_aomdec` | `aomdec` |
| `a12_ffmpeg` | `ffmpeg` with `libdav1d` and with `libaom-av1` |

A decoder missing from `PATH` skips its test and prints a notice. Run it once per feature set:

```sh
cargo test --release --test decode_back -- --nocapture
cargo test --release --no-default-features --test decode_back -- --nocapture
```

The test needs no feature flag, no network, and no media. It shares `scripts/gate/clips.rs` with the gate.

## The byte-identical gate

The gate proves that rav1r encodes the same bytes as upstream. Run it for every change that can reach the encoder output.

1. Build the reference encoder and write its packets:

   ```sh
   scripts/gate/build_upstream.sh
   ```

   - The script builds upstream at the pinned commit under `~/.cache/rav1r-gate/`.
   - It needs network access, and NASM on `x86_64`.
   - Upstream's `asm` feature stays off.
   - Upstream always builds the `asm` feature of `av-scenechange`. So the reference encoder sets `CPU_TARGET=RUST` and checks that the setting took effect.

2. Compare, once per feature set:

   ```sh
   export RAV1R_GATE_DIR=~/.cache/rav1r-gate/packets
   cargo test --no-default-features --test upstream_gate
   cargo test --test upstream_gate
   cargo test --features asm --test upstream_gate
   ```

With `RAV1R_GATE_DIR` unset, the test prints a skip notice and returns. CI runs it that way. `threading` builds compare with `threading` packets, and other builds compare with `sequential` packets.

`scripts/gate/clips.rs` holds the matrix and a seeded generator, and both sides compile it. No media enters the repository. Rerun the script after any change to that file.

## Merging upstream

1. Add the remote once: `git remote add upstream https://github.com/xiph/rav1e`.
2. Run `git fetch upstream` and `git merge upstream/master`.
3. Resolve a conflict at a removal site by keeping the removal. Drop any part that re-adds a removed binary, feature, or dependency.
4. Keep `rustfmt.toml`, and do not reformat files.
5. Apply upstream changes to `v_frame`, `av-scenechange`, or `av1-grain` by hand to `src/frame_buffer/`, `src/scenechange/`, or `src/api/config/grain.rs`.
6. Set the merged commit in `scripts/gate/Cargo.toml` and `scripts/gate/build_upstream.sh`, then run the gate.
7. Name any upstream change that alters the bitstream in the merge commit. Any other mismatch blocks the merge.
8. Run the [checks](#checks), with default features and with `asm`.

## Assembly level

`RAV1E_CPU_TARGET` lowers the assembly level at run time. `RAV1E_CPU_TARGET=rust` turns off all hand-written assembly. `src/cpu_features` lists the levels for each platform.

## Pull requests

- Add tests for every behavior change.
- Add a line to `CHANGELOG.md` for every user-visible change.
- Run the [checks](#checks) before you open the pull request.

## Releases

Each release needs a non-empty `## [X.Y.Z]` section in `CHANGELOG.md`. A pushed `v*` tag runs the checks in `Release Prepare`. The manual `Release` workflow publishes.
