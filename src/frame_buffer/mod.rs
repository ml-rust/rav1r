// Copyright (c) 2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! Frame, plane, and pixel types, inlined from `v_frame` 0.3.9.
//!
//! `v_frame` is BSD-2-Clause, and its source moves here with its headers.
//! `NOTICE` records the origin.

pub mod aligned;
pub mod frame;
pub mod math;
pub mod pixel;
pub mod plane;

pub mod prelude {
  pub use super::math::*;
  pub use super::pixel::*;
}
