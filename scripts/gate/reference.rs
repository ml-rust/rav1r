// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! Writes the reference packets of the byte-identical gate.
//!
//! Usage: `reference <output-directory>`

use rav1e::prelude as enc;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

#[path = "clips.rs"]
mod clips;

fn run() -> Result<(), String> {
  let out_dir = env::args_os()
    .nth(1)
    .map(PathBuf::from)
    .ok_or("missing output directory: run `reference <output-directory>`")?;

  // rav1e always builds av-scenechange with `asm`. This selects its Rust paths.
  // It runs before any thread starts.
  env::set_var("CPU_TARGET", "RUST");
  let level = av_scenechange::CpuFeatureLevel::default();
  if level != av_scenechange::CpuFeatureLevel::RUST {
    return Err(format!(
      "av-scenechange selected {level:?} with CPU_TARGET=RUST: check its version"
    ));
  }

  let out_dir = out_dir.join(clips::variant());
  fs::create_dir_all(&out_dir).map_err(|err| {
    format!("cannot create {}: {err}: check the path", out_dir.display())
  })?;

  for case in clips::matrix() {
    let path = out_dir.join(case.file_name());
    let packets = clips::encode_case(&case)
      .map_err(|err| format!("{}: {err}", case.file_name()))?;
    fs::write(&path, packets).map_err(|err| {
      format!("cannot write {}: {err}: check the path", path.display())
    })?;
  }
  Ok(())
}

fn main() -> ExitCode {
  match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      eprintln!("reference: {err}");
      ExitCode::FAILURE
    }
  }
}
