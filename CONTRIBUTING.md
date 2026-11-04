# Contributing to rav1r

rav1r is the AV1 encoder of the ml-rust media stack. It derives from rav1e and stays mergeable with it. [AGENTS.md](AGENTS.md) holds the placement rules and invariants.

## Toolchain

- Stable Rust, version 1.85 or later.
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
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --no-default-features --features asm -- -D warnings
cargo clippy --all-targets --no-default-features --features serialize -- -D warnings
cargo test --no-fail-fast
cargo test --no-default-features --no-fail-fast
cargo fmt --all --check
cargo deny --exclude-dev check
cargo doc --no-deps --no-default-features --features threading,serialize
```

Encode-decode tests need libaom and libdav1d:

```sh
sudo apt install libaom-dev libdav1d-dev
cargo test --release --features decode_test --lib -- test_encode_decode
cargo test --release --features decode_test_dav1d --lib -- test_encode_decode
```

## The byte-identical gate

The gate proves that rav1r encodes the same bytes as upstream. Run it for every change that can reach the encoder output.

1. Build the reference encoder and write its packets:

   ```sh
   scripts/gate/build_upstream.sh
   ```

   The script builds upstream at the pinned commit under `~/.cache/rav1r-gate/`, and it needs network access and NASM on `x86_64`. Upstream's `asm` feature stays off. Upstream always builds the `asm` feature of `av-scenechange`, so the reference encoder sets `CPU_TARGET=RUST` and checks that the setting took effect.

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

1. Run `git fetch upstream` and `git merge upstream/master`.
2. Resolve a conflict at a removal site by keeping the removal. Drop any part that re-adds a removed binary, feature, or dependency.
3. Apply upstream changes to `v_frame`, `av-scenechange`, or `av1-grain` by hand to `src/frame_buffer/`, `src/scenechange/`, or `src/api/config/grain.rs`.
4. Set the merged commit in `scripts/gate/Cargo.toml` and `scripts/gate/build_upstream.sh`, then run the gate.
5. Name any upstream change that alters the bitstream in the merge commit. Any other mismatch blocks the merge.

## Assembly level

`RAV1E_CPU_TARGET` lowers the assembly level at run time. `RAV1E_CPU_TARGET=rust` turns off all hand-written assembly. `src/cpu_features` lists the levels for each platform.

## Releases

Each release needs a non-empty `## [X.Y.Z]` section in `CHANGELOG.md`. A pushed `v*` tag runs the checks in `Release Prepare`. The manual `Release` workflow publishes.
