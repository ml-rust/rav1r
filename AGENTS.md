# rav1r — Agent Guidelines

AV1 encoder for the ml-rust stack. rav1r derives from rav1e (xiph/rav1e), base commit `31435de9d76fddd38f6dcc31d4014574cebb2092`. vidr depends on it as a library.

## Placement

- `src/`: the encoder library, as upstream lays it out.
- `src/par.rs`, `src/arg_enum.rs`, `src/frame_buffer/`, `src/scenechange/`, `src/api/config/grain.rs`: the only in-tree replacements for removed dependencies. Inlined files keep their source's notice, and `NOTICE` names each source.
- `scripts/gate/`: the reference encoder for the byte-identical gate. It is its own workspace, outside the published crate and outside `cargo deny`. Only its build needs NASM.
- vidr: frames, hardware paths, filter graph, CLI. NEVER add them here.
- containr: IVF, MP4, MKV and WebM. NEVER add a container writer or reader here.
- NEVER add a binary, a C API, a fuzz target, or a bench harness. They were removed on purpose.

## Invariants

- rav1r derives from rav1e. Keep every merge from `upstream` practical.
- NEVER reformat a file. NEVER run `cargo fmt` across the tree. Run `rustfmt` only on a file you rewrote substantially.
- Keep `rustfmt.toml`: 2 spaces, width 79.
- Keep each edit minimal at its site. Change the lines that need it and no others.
- Keep `LICENSE`, `PATENTS`, and every per-file header unchanged. Keep the assembly headers (VideoLAN, dav1d authors, Two Orioles, Martin Storsjo, and the rest) in place.
- Keep `NOTICE` current: base commit, rav1e attribution, pointers to `LICENSE` and `PATENTS`.
- NEVER touch `src/asm/`, `src/x86/`, `src/arm/`, or `src/ext/` for style or cleanup. A change there needs a stated reason and passes the byte-identical gate.
- The pure-Rust build always works. Run it with `--no-default-features` and with default features. NASM and a C compiler are NEVER required.
- The `asm` feature is off by default. It enables `nasm-rs`, `cc`, `libc`, and `pastey`. NEVER add `asm` to `default`.
- Keep 12-bit, 4:2:2 and 4:4:4 in the encoder, even though vidr does not call them. Removal makes merges larger.
- Every optional capability is a Cargo feature. The default build is `threading` only.
- With `threading` off, `src/par.rs` runs the same code on the calling thread.
- A dependency must have a release or commit in the last year. Otherwise write our own or remove it.
- `cargo deny --exclude-dev check` must pass. NEVER add a git dependency outside `scripts/gate/`.
- Output stays bit-exact with upstream for the same input and config.

## Merging upstream

- Fetch with `git fetch upstream` and merge `upstream/master`. Resolve a conflict at a removal site by keeping the removal.
- Byte-identical gate: `scripts/gate/build_upstream.sh` writes reference packets from upstream at the base commit. Bump its pinned commit in the same merge, then rerun it.
- The packet bytes must match. A mismatch is acceptable only when the upstream change alters the bitstream. Name that change in the merge commit.
- Run the gate with no features, default features, and `asm`.
- `scripts/gate/clips.rs` is shared by both sides. Rerun the script after any change to it.
- NEVER merge an upstream change that re-adds a removed binary, feature, or dependency. Drop that part.

## Commands

- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets --no-default-features -- -D warnings`
- `cargo nextest run --no-fail-fast`
- `cargo nextest run --no-default-features --no-fail-fast`
- `cargo fmt --all --check`
- `cargo deny --exclude-dev check`
- `cargo test --doc`
- `scripts/gate/build_upstream.sh`, then `RAV1R_GATE_DIR=~/.cache/rav1r-gate/packets cargo test --test upstream_gate -- --nocapture`
