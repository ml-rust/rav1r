#!/usr/bin/env bash
# Build the reference encoder from upstream rav1e and write its packets.
#
#   scripts/gate/build_upstream.sh [output-dir]
#
# output-dir defaults to ~/.cache/rav1r-gate/packets. All build output stays
# under ~/.cache/rav1r-gate. Pass the output directory to the test:
#
#   RAV1R_GATE_DIR=~/.cache/rav1r-gate/packets cargo test --test upstream_gate
#
# Assembly stays off on the upstream side:
#   - `default-features = false` leaves out the `asm` feature of rav1e.
#   - rav1e always enables the `asm` feature of av-scenechange, so the build
#     needs NASM on x86_64. CPU_TARGET=RUST selects its Rust paths at run time,
#     and the reference encoder exits with an error if it does not.

set -euo pipefail

readonly UPSTREAM_COMMIT=31435de9d76fddd38f6dcc31d4014574cebb2092

SOURCE_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CACHE_DIR="${HOME}/.cache/rav1r-gate"
OUT_DIR="${1:-${CACHE_DIR}/packets}"
WORK_DIR="${CACHE_DIR}/reference"

if [[ "$(uname -m)" == "x86_64" ]] && ! command -v nasm > /dev/null; then
    echo "error: nasm not found: install NASM 2.15 or newer, then rerun" >&2
    exit 1
fi

export CARGO_HOME="${CACHE_DIR}/cargo-home"
export CARGO_TARGET_DIR="${CACHE_DIR}/target"
export CPU_TARGET=RUST

mkdir -p "${WORK_DIR}" "${OUT_DIR}"
cp "${SOURCE_DIR}/Cargo.toml" "${SOURCE_DIR}/clips.rs" "${SOURCE_DIR}/reference.rs" "${WORK_DIR}/"

# Cargo checks out the commit named in Cargo.toml again after this.
rm -rf "${CARGO_HOME}/git/checkouts"

cargo build --release --manifest-path "${WORK_DIR}/Cargo.toml"
cp "${CARGO_TARGET_DIR}/release/reference" "${WORK_DIR}/reference-sequential"
cargo build --release --manifest-path "${WORK_DIR}/Cargo.toml" --features threading
cp "${CARGO_TARGET_DIR}/release/reference" "${WORK_DIR}/reference-threading"

CHECKOUT=$(find "${CARGO_HOME}/git/checkouts" -mindepth 2 -maxdepth 2 -type d -name "${UPSTREAM_COMMIT:0:7}" -print -quit)
if [[ -z "${CHECKOUT}" ]]; then
    echo "error: no checkout of ${UPSTREAM_COMMIT} under ${CARGO_HOME}/git/checkouts: check the rev in Cargo.toml" >&2
    exit 1
fi
if [[ "$(git -C "${CHECKOUT}" rev-parse HEAD)" != "${UPSTREAM_COMMIT}" ]]; then
    echo "error: ${CHECKOUT} is not at ${UPSTREAM_COMMIT}: remove ${CACHE_DIR} and rerun" >&2
    exit 1
fi
if [[ -n "$(git -C "${CHECKOUT}" status --porcelain --untracked-files=no)" ]]; then
    echo "error: ${CHECKOUT} has local changes: remove ${CACHE_DIR} and rerun" >&2
    exit 1
fi

"${WORK_DIR}/reference-sequential" "${OUT_DIR}"
"${WORK_DIR}/reference-threading" "${OUT_DIR}"

echo "Reference packets: ${OUT_DIR}"
