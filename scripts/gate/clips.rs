// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! Clips, configs, and the encode loop of the byte-identical gate.
//!
//! `tests/upstream_gate.rs` and `scripts/gate/reference.rs` both include this
//! file. Each one names its encoder as `enc`: rav1r in the test, upstream rav1e
//! in the reference. A change here changes both sides, so rerun
//! `scripts/gate/build_upstream.sh` after it.
//!
//! `tests/decode_back.rs` also includes this file, for `Case` and `fill_frame`.

use super::enc::{
  ChromaSampling, Config, Context, EncoderConfig, EncoderStatus, Pixel,
  Rational,
};

/// The worker count both sides use when `threading` is on.
const THREADS: usize = 2;

/// One encode: a clip and the config that encodes it.
pub struct Case {
  pub width: usize,
  pub height: usize,
  pub bit_depth: usize,
  pub speed: u8,
  pub low_latency: bool,
  /// The target bitrate. Zero selects quantizer mode.
  pub bitrate: i32,
  pub tile_cols: usize,
  pub tile_rows: usize,
  pub frames: u32,
  pub seed: u32,
}

impl Case {
  /// Returns the name of this case's packet file, without a directory.
  pub fn file_name(&self) -> String {
    format!(
      "{}x{}_{}bit_speed{}_{}_{}_tiles{}x{}.bin",
      self.width,
      self.height,
      self.bit_depth,
      self.speed,
      if self.low_latency { "lowlatency" } else { "reorder" },
      if self.bitrate == 0 { "quantizer" } else { "bitrate" },
      self.tile_cols,
      self.tile_rows,
    )
  }
}

/// Returns the directory name for the current `threading` setting.
pub fn variant() -> &'static str {
  if cfg!(feature = "threading") {
    "threading"
  } else {
    "sequential"
  }
}

/// Returns every case of the gate.
pub fn matrix() -> Vec<Case> {
  let mut cases = Vec::new();
  let mut add =
    |width, height, bit_depth, speed, low_latency, bitrate, tiles| {
      let seed = cases.len() as u32 + 1;
      cases.push(Case {
        width,
        height,
        bit_depth,
        speed,
        low_latency,
        bitrate,
        tile_cols: tiles,
        tile_rows: tiles,
        frames: 8,
        seed,
      });
    };

  for bit_depth in [8, 10] {
    // Speed, low latency, and rate control.
    for speed in [0, 4, 6, 10] {
      for low_latency in [false, true] {
        for bitrate in [0, 100] {
          add(48, 32, bit_depth, speed, low_latency, bitrate, 0);
        }
      }
    }
    for speed in [6, 10] {
      // Odd sizes.
      add(33, 17, bit_depth, speed, false, 0, 0);
      add(65, 49, bit_depth, speed, true, 0, 0);
      // Two tile columns and two tile rows.
      add(130, 130, bit_depth, speed, false, 0, 2);
    }
    // A short edge above 240 selects the downscaling path of fast detection.
    add(256, 256, bit_depth, 10, false, 0, 0);
  }
  cases
}

/// Mixes a value into a well-distributed one with wrapping arithmetic only.
fn mix(mut value: u32) -> u32 {
  value ^= value >> 16;
  value = value.wrapping_mul(0x9e37_79b1);
  value ^= value >> 13;
  value = value.wrapping_mul(0x85eb_ca77);
  value ^ (value >> 16)
}

/// Returns one 8-bit sample of a clip.
///
/// A blocky texture moves right by one pixel per frame, with noise on top.
/// The texture changes in the middle of the clip, which is a scene cut.
fn sample(case: &Case, frame: u32, plane: u32, x: u32, y: u32) -> u32 {
  let scene = u32::from(frame >= case.frames / 2);
  let base = case.seed.wrapping_mul(31).wrapping_add(scene * 1009 + plane * 7);
  let block = ((x + frame) / 4).wrapping_mul(0x0001_0001) ^ (y / 4);
  let texture = mix(base ^ block) & 0xff;
  let noise = mix(base ^ (frame << 24) ^ (x << 12) ^ y) & 0x1f;
  (texture * 7 / 8) + noise
}

/// Fills `frame` with frame number `index` of the clip.
pub fn fill_frame<T: Pixel>(
  case: &Case, frame: &mut super::enc::Frame<T>, index: u32,
) {
  let bytes_per_sample = if case.bit_depth > 8 { 2 } else { 1 };
  for (plane_index, plane) in frame.planes.iter_mut().enumerate() {
    let (width, height) = (plane.cfg.width, plane.cfg.height);
    let mut bytes = Vec::with_capacity(width * height * bytes_per_sample);
    for y in 0..height as u32 {
      for x in 0..width as u32 {
        let value = sample(case, index, plane_index as u32, x, y);
        if bytes_per_sample == 1 {
          bytes.push(value as u8);
        } else {
          let high = (value << 2) | (x & 3);
          bytes.extend_from_slice(&(high as u16).to_le_bytes());
        }
      }
    }
    plane.copy_from_raw_u8(&bytes, width * bytes_per_sample, bytes_per_sample);
  }
}

fn encode<T: Pixel>(case: &Case) -> Result<Vec<u8>, String> {
  let enc = EncoderConfig {
    width: case.width,
    height: case.height,
    bit_depth: case.bit_depth,
    chroma_sampling: ChromaSampling::Cs420,
    time_base: Rational { num: 1, den: 30 },
    low_latency: case.low_latency,
    quantizer: 100,
    bitrate: case.bitrate,
    tile_cols: case.tile_cols,
    tile_rows: case.tile_rows,
    min_key_frame_interval: 2,
    max_key_frame_interval: 30,
    ..EncoderConfig::with_speed_preset(case.speed)
  };
  let cfg = Config::new().with_encoder_config(enc).with_threads(THREADS);
  let mut ctx: Context<T> =
    cfg.new_context().map_err(|err| format!("invalid config: {err:?}"))?;

  let mut packets = Vec::new();
  let mut next_frame = 0;
  loop {
    match ctx.receive_packet() {
      Ok(packet) => {
        packets.extend_from_slice(&packet.input_frameno.to_le_bytes());
        packets.extend_from_slice(&(packet.data.len() as u64).to_le_bytes());
        packets.extend_from_slice(&packet.data);
      }
      Err(EncoderStatus::Encoded) => {}
      Err(EncoderStatus::LimitReached) => return Ok(packets),
      Err(EncoderStatus::NeedMoreData) if next_frame < case.frames => {
        let mut frame = ctx.new_frame();
        fill_frame(case, &mut frame, next_frame);
        ctx
          .send_frame(frame)
          .map_err(|err| format!("send_frame failed: {err:?}"))?;
        next_frame += 1;
      }
      Err(EncoderStatus::NeedMoreData) => ctx.flush(),
      Err(err) => return Err(format!("receive_packet failed: {err:?}")),
    }
  }
}

/// Encodes a case and returns its packets.
///
/// Each packet is its input frame number and its byte length, both as
/// little-endian `u64`, then its bytes.
pub fn encode_case(case: &Case) -> Result<Vec<u8>, String> {
  if case.bit_depth > 8 {
    encode::<u16>(case)
  } else {
    encode::<u8>(case)
  }
}
