// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! Decode-back: independent decoders reproduce the encoder's reconstruction.
//!
//! Each case encodes a seeded synthetic clip and writes the packets to an IVF
//! file under `CARGO_TARGET_TMPDIR`. A decoder command line tool decodes the
//! file to raw planes. The test compares every decoded plane byte for byte
//! with `Packet.rec`, in output order. The encoder emits no film grain.
//!
//! A decoder that is absent from `PATH` skips its test with a printed notice.
//! Tests:
//!
//! - `a10_dav1d`: the `dav1d` command line tool.
//! - `a11_aomdec`: the `aomdec` command line tool.
//! - `a12_ffmpeg`: `ffmpeg` with the `libdav1d` and `libaom-av1` decoders.

use rav1r::prelude as enc;

use enc::{
  ChromaSampling, Config, Context, EncoderConfig, EncoderStatus, Pixel,
  Rational,
};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
#[path = "../scripts/gate/clips.rs"]
mod clips;

/// One encode: a gate clip case and the encoder config around it.
struct Spec {
  case: clips::Case,
  quantizer: usize,
  threads: usize,
  min_keyint: u64,
  max_keyint: u64,
}

impl Spec {
  fn new(width: usize, height: usize, bit_depth: usize, speed: u8) -> Self {
    Spec {
      case: clips::Case {
        width,
        height,
        bit_depth,
        speed,
        low_latency: false,
        bitrate: 0,
        tile_cols: 0,
        tile_rows: 0,
        frames: 6,
        seed: 0,
      },
      quantizer: 100,
      threads: 2,
      min_keyint: 2,
      max_keyint: 30,
    }
  }

  fn low_latency(mut self) -> Self {
    self.case.low_latency = true;
    self
  }

  fn quantizer(mut self, quantizer: usize) -> Self {
    self.quantizer = quantizer;
    self
  }

  fn bitrate(mut self, bitrate: i32) -> Self {
    self.case.bitrate = bitrate;
    self
  }

  fn tiles(mut self, cols: usize, rows: usize) -> Self {
    self.case.tile_cols = cols;
    self.case.tile_rows = rows;
    self
  }

  fn threads(mut self, threads: usize) -> Self {
    self.threads = threads;
    self
  }

  fn keyint(mut self, min: u64, max: u64) -> Self {
    self.min_keyint = min;
    self.max_keyint = max;
    self
  }

  fn frames(mut self, frames: u32) -> Self {
    self.case.frames = frames;
    self
  }

  fn name(&self) -> String {
    let case = &self.case;
    let rate = if case.bitrate == 0 {
      format!("q{}", self.quantizer)
    } else {
      format!("bitrate{}", case.bitrate)
    };
    format!(
      "{}x{}_{}bit_speed{}_{}_{}_tiles{}x{}_threads{}_keyint{}-{}_{}frames",
      case.width,
      case.height,
      case.bit_depth,
      case.speed,
      if case.low_latency { "lowlatency" } else { "reorder" },
      rate,
      case.tile_cols,
      case.tile_rows,
      self.threads,
      self.min_keyint,
      self.max_keyint,
      case.frames,
    )
  }
}

/// Returns every case. Each case has its own name and its own seed.
fn matrix() -> Vec<Spec> {
  let mut specs = Vec::new();
  for bit_depth in [8, 10] {
    let base = |speed: u8| Spec::new(48, 32, bit_depth, speed);

    // Every speed preset, with frame reordering.
    for speed in 0..=10 {
      specs.push(base(speed));
    }
    // Low latency on.
    for speed in [0, 3, 6, 10] {
      specs.push(base(speed).low_latency());
    }
    // Quantizer mode. Quantizer 0 is lossless.
    for quantizer in [0, 40, 180, 255] {
      for speed in [4, 10] {
        specs.push(base(speed).quantizer(quantizer));
      }
    }
    // Bitrate mode.
    for bitrate in [50, 100, 800] {
      specs.push(base(6).bitrate(bitrate));
      specs.push(base(6).low_latency().bitrate(bitrate));
    }
    specs.push(base(0).bitrate(100));
    specs.push(base(10).bitrate(100));
    // Odd sizes.
    for (width, height) in [(33, 17), (17, 31), (65, 49), (129, 67)] {
      specs.push(Spec::new(width, height, bit_depth, 6));
      specs.push(Spec::new(width, height, bit_depth, 10).low_latency());
    }
    // Tiles, and worker counts on a tiled clip. Zero selects the global pool.
    for threads in [0, 1, 2, 4] {
      specs.push(
        Spec::new(130, 130, bit_depth, 6)
          .tiles(2, 2)
          .threads(threads)
          .frames(4),
      );
    }
    specs.push(Spec::new(130, 130, bit_depth, 10).tiles(2, 2).frames(4));
    specs.push(
      Spec::new(130, 130, bit_depth, 6).low_latency().tiles(2, 2).frames(4),
    );
    specs.push(Spec::new(256, 64, bit_depth, 10).tiles(4, 1).frames(4));
    specs.push(Spec::new(64, 192, bit_depth, 6).tiles(1, 2).frames(4));
    // Worker counts on an untiled clip.
    for threads in [0, 1, 4] {
      specs.push(base(6).threads(threads));
    }
    // Several key frames, with and without reordering.
    specs.push(base(6).keyint(1, 3).frames(8));
    specs.push(base(6).low_latency().keyint(1, 3).frames(8));
  }

  let mut names = HashSet::new();
  for (index, spec) in specs.iter_mut().enumerate() {
    spec.case.seed = index as u32 + 1;
    assert!(names.insert(spec.name()), "duplicate case {}", spec.name());
  }
  specs
}

/// A finished encode: the IVF file and the raw planes of every `Packet.rec`.
struct Encoded {
  ivf: Vec<u8>,
  expected: Vec<u8>,
}

/// Appends the visible samples of every plane. A reconstruction plane is
/// padded to a multiple of 8, so each plane crops to the clip's size. A sample
/// above 8 bits takes two little-endian bytes.
fn append_frame<T: Pixel>(
  out: &mut Vec<u8>, frame: &enc::Frame<T>, bit_depth: usize,
  (width, height): (usize, usize),
) {
  let chroma = (width.div_ceil(2), height.div_ceil(2));
  let sizes = [(width, height), chroma, chroma];
  for (plane, (columns, rows)) in frame.planes.iter().zip(sizes) {
    for row in plane.rows_iter().take(rows) {
      for &pixel in &row[..columns] {
        let value: u32 = pixel.into();
        if bit_depth > 8 {
          out.extend_from_slice(&(value as u16).to_le_bytes());
        } else {
          out.push(value as u8);
        }
      }
    }
  }
}

/// Returns an IVF file. The header holds the size and a 1/30 time base.
fn ivf_file(spec: &Spec, count: u32, frames: &[u8]) -> Vec<u8> {
  let mut file = Vec::with_capacity(32 + frames.len());
  file.extend_from_slice(b"DKIF");
  file.extend_from_slice(&0u16.to_le_bytes());
  file.extend_from_slice(&32u16.to_le_bytes());
  file.extend_from_slice(b"AV01");
  file.extend_from_slice(&(spec.case.width as u16).to_le_bytes());
  file.extend_from_slice(&(spec.case.height as u16).to_le_bytes());
  file.extend_from_slice(&30u32.to_le_bytes());
  file.extend_from_slice(&1u32.to_le_bytes());
  file.extend_from_slice(&count.to_le_bytes());
  file.extend_from_slice(&0u32.to_le_bytes());
  file.extend_from_slice(frames);
  file
}

fn encode<T: Pixel>(spec: &Spec) -> Result<Encoded, String> {
  let case = &spec.case;
  let config = EncoderConfig {
    width: case.width,
    height: case.height,
    bit_depth: case.bit_depth,
    chroma_sampling: ChromaSampling::Cs420,
    time_base: Rational { num: 1, den: 30 },
    low_latency: case.low_latency,
    quantizer: spec.quantizer,
    bitrate: case.bitrate,
    tile_cols: case.tile_cols,
    tile_rows: case.tile_rows,
    min_key_frame_interval: spec.min_keyint,
    max_key_frame_interval: spec.max_keyint,
    ..EncoderConfig::with_speed_preset(case.speed)
  };
  let config =
    Config::new().with_encoder_config(config).with_threads(spec.threads);
  let mut ctx: Context<T> =
    config.new_context().map_err(|err| format!("invalid config: {err:?}"))?;

  let mut frames = Vec::new();
  let mut expected = Vec::new();
  let mut count = 0u32;
  let mut next_frame = 0;
  loop {
    match ctx.receive_packet() {
      Ok(packet) => {
        let rec = packet
          .rec
          .as_ref()
          .ok_or_else(|| "a packet has no reconstruction".to_string())?;
        append_frame(
          &mut expected,
          rec,
          case.bit_depth,
          (case.width, case.height),
        );
        frames.extend_from_slice(&(packet.data.len() as u32).to_le_bytes());
        frames.extend_from_slice(&packet.input_frameno.to_le_bytes());
        frames.extend_from_slice(&packet.data);
        count += 1;
      }
      Err(EncoderStatus::Encoded) => {}
      Err(EncoderStatus::LimitReached) => break,
      Err(EncoderStatus::NeedMoreData) if next_frame < case.frames => {
        let mut frame = ctx.new_frame();
        clips::fill_frame(case, &mut frame, next_frame);
        ctx
          .send_frame(frame)
          .map_err(|err| format!("send_frame failed: {err:?}"))?;
        next_frame += 1;
      }
      Err(EncoderStatus::NeedMoreData) => ctx.flush(),
      Err(err) => return Err(format!("receive_packet failed: {err:?}")),
    }
  }
  if count != case.frames {
    return Err(format!(
      "the encoder emitted {count} packets for {} frames",
      case.frames
    ));
  }
  Ok(Encoded { ivf: ivf_file(spec, count, &frames), expected })
}

fn encode_spec(spec: &Spec) -> Result<Encoded, String> {
  if spec.case.bit_depth > 8 {
    encode::<u16>(spec)
  } else {
    encode::<u8>(spec)
  }
}

/// A command line decoder that writes raw planes.
#[derive(Clone, Copy)]
enum Decoder {
  Dav1d,
  Aomdec,
  FfmpegDav1d,
  FfmpegAom,
}

impl Decoder {
  fn name(self) -> &'static str {
    match self {
      Decoder::Dav1d => "dav1d",
      Decoder::Aomdec => "aomdec",
      Decoder::FfmpegDav1d => "ffmpeg-libdav1d",
      Decoder::FfmpegAom => "ffmpeg-libaom-av1",
    }
  }

  /// Returns the reason this decoder cannot run, or `None` when it can.
  fn unavailable(self) -> Option<String> {
    let probe = match self {
      Decoder::Dav1d => Command::new("dav1d").arg("--version").output(),
      Decoder::Aomdec => Command::new("aomdec").arg("--help").output(),
      Decoder::FfmpegDav1d | Decoder::FfmpegAom => {
        Command::new("ffmpeg").args(["-hide_banner", "-decoders"]).output()
      }
    };
    let output = match probe {
      Ok(output) => output,
      Err(err) => return Some(format!("the program does not start: {err}")),
    };
    let library = match self {
      Decoder::FfmpegDav1d => "libdav1d",
      Decoder::FfmpegAom => "libaom-av1",
      _ => return None,
    };
    let listing = String::from_utf8_lossy(&output.stdout);
    if listing.split_whitespace().any(|word| word == library) {
      None
    } else {
      Some(format!("ffmpeg has no {library} decoder"))
    }
  }

  fn command(self, input: &Path, output: &Path, bit_depth: usize) -> Command {
    match self {
      Decoder::Dav1d => {
        let mut command = Command::new("dav1d");
        command
          .args(["--filmgrain", "0", "-i"])
          .arg(input)
          .arg("-o")
          .arg(output);
        command
      }
      Decoder::Aomdec => {
        let mut command = Command::new("aomdec");
        command
          .args(["--rawvideo", "--skip-film-grain", "-o"])
          .arg(output)
          .arg(input);
        command
      }
      Decoder::FfmpegDav1d | Decoder::FfmpegAom => {
        let codec = match self {
          Decoder::FfmpegDav1d => "libdav1d",
          _ => "libaom-av1",
        };
        let pix_fmt = if bit_depth > 8 { "yuv420p10le" } else { "yuv420p" };
        let mut command = Command::new("ffmpeg");
        command
          .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y"])
          .args(["-c:v", codec]);
        if let Decoder::FfmpegDav1d = self {
          // Keep film grain out of the output.
          command.args(["-export_side_data", "film_grain"]);
        }
        command
          .arg("-i")
          .arg(input)
          .args(["-fps_mode", "passthrough", "-f", "rawvideo"])
          .args(["-pix_fmt", pix_fmt])
          .arg(output);
        command
      }
    }
  }
}

/// Describes where `offset` falls in the raw output of `spec`.
fn locate(spec: &Spec, offset: usize) -> String {
  let case = &spec.case;
  let bytes = if case.bit_depth > 8 { 2 } else { 1 };
  let (width, height) = (case.width, case.height);
  let (chroma_width, chroma_height) = (width.div_ceil(2), height.div_ceil(2));
  let frame_bytes =
    (width * height + 2 * chroma_width * chroma_height) * bytes;
  let frame = offset / frame_bytes;
  let mut sample = offset % frame_bytes / bytes;
  let planes = [
    (width, height),
    (chroma_width, chroma_height),
    (chroma_width, chroma_height),
  ];
  for (plane, (plane_width, plane_height)) in planes.into_iter().enumerate() {
    if sample < plane_width * plane_height {
      return format!(
        "frame {frame} plane {plane} row {} column {}",
        sample / plane_width,
        sample % plane_width
      );
    }
    sample -= plane_width * plane_height;
  }
  format!("frame {frame}")
}

fn compare(
  spec: &Spec, decoded: &[u8], expected: &[u8],
) -> Result<(), String> {
  let first_difference =
    decoded.iter().zip(expected).position(|(left, right)| left != right);
  match first_difference {
    Some(offset) => Err(format!(
      "decoded planes differ from the reconstruction at {}",
      locate(spec, offset)
    )),
    None if decoded.len() != expected.len() => Err(format!(
      "decoded {} bytes against {} reconstructed bytes",
      decoded.len(),
      expected.len()
    )),
    None => Ok(()),
  }
}

fn decode_and_compare(
  decoder: Decoder, spec: &Spec, ivf_path: &Path, yuv_path: &Path,
  expected: &[u8],
) -> Result<(), String> {
  let _ = fs::remove_file(yuv_path);
  let output = decoder
    .command(ivf_path, yuv_path, spec.case.bit_depth)
    .output()
    .map_err(|err| format!("cannot run {}: {err}", decoder.name()))?;
  if !output.status.success() {
    return Err(format!(
      "{} exited with {}: {}",
      decoder.name(),
      output.status,
      String::from_utf8_lossy(&output.stderr).trim()
    ));
  }
  let decoded = fs::read(yuv_path)
    .map_err(|err| format!("cannot read {}: {err}", yuv_path.display()))?;
  compare(spec, &decoded, expected)?;
  fs::remove_file(yuv_path)
    .map_err(|err| format!("cannot remove {}: {err}", yuv_path.display()))
}

/// Runs the whole matrix through each decoder that exists on this machine.
fn run(test: &str, decoders: &[Decoder]) {
  let mut present = Vec::new();
  for &decoder in decoders {
    match decoder.unavailable() {
      Some(reason) => eprintln!(
        "decode_back: skipped: {} is unavailable: {reason}",
        decoder.name()
      ),
      None => present.push(decoder),
    }
  }
  if present.is_empty() {
    return;
  }

  let dir =
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("decode_back").join(test);
  fs::create_dir_all(&dir).unwrap_or_else(|err| {
    panic!("cannot create {}: {err}: check the path", dir.display())
  });

  let mut errors = Vec::new();
  for spec in matrix() {
    let name = spec.name();
    let encoded = match encode_spec(&spec) {
      Ok(encoded) => encoded,
      Err(err) => {
        errors.push(format!("{name}: encode failed: {err}"));
        continue;
      }
    };
    let ivf_path = dir.join(format!("{name}.ivf"));
    fs::write(&ivf_path, &encoded.ivf).unwrap_or_else(|err| {
      panic!("cannot write {}: {err}: check the path", ivf_path.display())
    });
    for &decoder in &present {
      let yuv_path = dir.join(format!("{name}.{}.yuv", decoder.name()));
      if let Err(err) = decode_and_compare(
        decoder,
        &spec,
        &ivf_path,
        &yuv_path,
        &encoded.expected,
      ) {
        errors.push(format!("{name}: {}: {err}", decoder.name()));
      }
    }
  }

  assert!(
    errors.is_empty(),
    "decoded output differs from the encoder in {} cases:\n{}",
    errors.len(),
    errors.join("\n")
  );
}

#[test]
fn a10_dav1d() {
  run("a10_dav1d", &[Decoder::Dav1d]);
}

#[test]
fn a11_aomdec() {
  run("a11_aomdec", &[Decoder::Aomdec]);
}

#[test]
fn a12_ffmpeg() {
  run("a12_ffmpeg", &[Decoder::FfmpegDav1d, Decoder::FfmpegAom]);
}
