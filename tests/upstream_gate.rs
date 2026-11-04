// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! The byte-identical gate: rav1r encodes the same bytes as upstream rav1e.
//!
//! `scripts/gate/build_upstream.sh` writes the reference packets. Set
//! `RAV1R_GATE_DIR` to its output directory to run this test.

use rav1r::prelude as enc;
use std::fs;
use std::path::PathBuf;

#[path = "../scripts/gate/clips.rs"]
mod clips;

#[test]
fn packets_match_upstream() {
  let Some(gate_dir) = std::env::var_os("RAV1R_GATE_DIR") else {
    eprintln!("upstream_gate: skipped: RAV1R_GATE_DIR is unset");
    return;
  };
  let dir = PathBuf::from(gate_dir).join(clips::variant());

  let mut mismatches = Vec::new();
  for case in clips::matrix() {
    let name = case.file_name();
    let path = dir.join(&name);
    let expected = fs::read(&path).unwrap_or_else(|err| {
      panic!(
        "cannot read {}: {err}: run scripts/gate/build_upstream.sh",
        path.display()
      )
    });
    let actual = clips::encode_case(&case)
      .unwrap_or_else(|err| panic!("{name}: encode failed: {err}"));
    if actual != expected {
      let offset = actual
        .iter()
        .zip(&expected)
        .position(|(a, b)| a != b)
        .unwrap_or(actual.len().min(expected.len()));
      mismatches.push(format!(
        "{name}: {} bytes against {} upstream, first difference at {offset}",
        actual.len(),
        expected.len()
      ));
    }
  }

  assert!(
    mismatches.is_empty(),
    "packets differ from upstream in {} cases:\n{}",
    mismatches.len(),
    mismatches.join("\n")
  );
}
