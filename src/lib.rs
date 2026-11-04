// Copyright (c) 2017-2022, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! rav1r is an [AV1] video encoder. It is designed to eventually cover all use
//! cases, though in its current form it is most suitable for cases where
//! libaom (the reference encoder) is too slow.
//!
//! ## Features
//!
//! * Intra and inter frames
//! * 64x64 superblocks
//! * 4x4 to 64x64 RDO-selected square and 2:1/1:2 rectangular blocks
//! * DC, H, V, Paeth, smooth, and a subset of directional prediction modes
//! * DCT, (FLIP-)ADST and identity transforms (up to 64x64, 16x16 and 32x32
//!   respectively)
//! * 8-, 10- and 12-bit depth color
//! * 4:2:0 (full support), 4:2:2 and 4:4:4 (limited) chroma sampling
//! * Variable speed settings
//! * Near real-time encoding at high speed levels
//!
//! ## Usage
//!
//! Encoding is done through the [`Context`] struct. Examples on
//! [`Context::receive_packet`] show how to create a [`Context`], send frames
//! into it and receive packets of encoded data.
//!
//! [AV1]: https://aomediacodec.github.io/av1-spec/av1-spec.pdf
//! [`Context`]: struct.Context.html
//! [`Context::receive_packet`]: struct.Context.html#method.receive_packet

#![allow(missing_abi)]
#![allow(unused_unsafe)]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub use crate::api::color;
pub use crate::api::{
  Config, Context, EncoderConfig, EncoderStatus, InvalidConfig, Packet,
};
use crate::encoder::*;
pub use crate::frame::Frame;
pub use crate::util::{CastFromPrimitive, Pixel, PixelType};

#[macro_use]
mod arg_enum;
#[macro_use]
mod transform;
#[macro_use]
mod cpu_features;

mod activity;
pub(crate) mod asm;
mod dist;
mod ec;
mod partition;
mod predict;
mod quantize;
mod rdo;
mod rdo_tables;
#[macro_use]
mod util;
mod cdef;
#[doc(hidden)]
pub mod context;
mod deblock;
mod encoder;
mod entropymode;
mod levels;
mod lrf;
mod mc;
mod me;
mod rate;
mod recon_intra;
mod scan_order;
mod segmentation;
mod stats;
#[doc(hidden)]
pub mod tiling;
mod token_cdfs;

mod api;
mod frame;
#[doc(hidden)]
pub mod frame_buffer;
mod header;
mod scenechange;
#[doc(hidden)]
pub mod par;

/// Commonly used types and traits.
pub mod prelude {
  pub use crate::api::*;
  pub use crate::encoder::{Sequence, Tune};
  pub use crate::frame::{
    Frame, FrameParameters, FrameTypeOverride, Plane, PlaneConfig,
  };
  pub use crate::partition::BlockSize;
  pub use crate::predict::PredictionMode;
  pub use crate::transform::TxType;
  pub use crate::util::{CastFromPrimitive, Pixel, PixelType};
}

/// Basic data structures
pub mod data {
  pub use crate::api::{
    ChromaticityPoint, EncoderStatus, FrameType, Packet, Rational,
  };
  pub use crate::frame::{Frame, FrameParameters};
  pub use crate::stats::EncoderStats;
  pub use crate::util::{CastFromPrimitive, Pixel, PixelType};
}

/// Encoder configuration and settings
pub mod config {
  pub use crate::api::config::{
    GrainTableSegment, NoiseGenArgs, TransferFunction, NUM_UV_COEFFS,
    NUM_UV_POINTS, NUM_Y_COEFFS, NUM_Y_POINTS,
  };
  pub use crate::api::{
    Config, EncoderConfig, InvalidConfig, PredictionModesSetting,
    RateControlConfig, RateControlError, RateControlSummary, SpeedSettings,
  };
  pub use crate::cpu_features::CpuFeatureLevel;
}

/// Version information
///
/// The information is recovered from `Cargo.toml`.
///
/// ```
/// use rav1r::version;
/// use semver::Version;
///
/// let major = version::major();
/// let minor = version::minor();
/// let patch = version::patch();
///
/// let short = version::short();
///
/// let v1 = Version::new(major, minor, patch);
/// let v2 = Version::parse(&short).unwrap();
///
/// assert_eq!(v1.major, v2.major);
/// ```
pub mod version {
  /// Major version component
  ///
  /// It is increased every time a release presents a incompatible API change.
  ///
  /// # Panics
  ///
  /// Will panic if package is not built with Cargo,
  /// or if the package version is not a valid triplet of integers.
  pub fn major() -> u64 {
    env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap()
  }
  /// Minor version component
  ///
  /// It is increased every time a release presents new functionalities are added
  /// in a backwards-compatible manner.
  ///
  /// # Panics
  ///
  /// Will panic if package is not built with Cargo,
  /// or if the package version is not a valid triplet of integers.
  pub fn minor() -> u64 {
    env!("CARGO_PKG_VERSION_MINOR").parse().unwrap()
  }
  /// Patch version component
  ///
  /// It is increased every time a release provides only backwards-compatible bugfixes.
  ///
  /// # Panics
  ///
  /// Will panic if package is not built with Cargo,
  /// or if the package version is not a valid triplet of integers.
  pub fn patch() -> u64 {
    env!("CARGO_PKG_VERSION_PATCH").parse().unwrap()
  }

  /// Version information as presented in `[package]` `version`.
  ///
  /// e.g. `0.1.0`
  ///
  /// Can be parsed by [semver](https://crates.io/crates/semver).
  pub fn short() -> String {
    env!("CARGO_PKG_VERSION").to_string()
  }

  /// Version information as presented in `[package]` `version`.
  ///
  /// It equals [`short`]. No commit hash is included.
  pub fn long() -> String {
    short()
  }

  /// Version information as presented in `[package]` `version`.
  ///
  /// It equals [`short`]. No `git describe` output is included.
  pub fn full() -> String {
    short()
  }
}
#[cfg(all(
  test,
  any(feature = "decode_test", feature = "decode_test_dav1d")
))]
mod test_encode_decode;
