// Derived from av-scenechange 0.14.1
// (https://github.com/rust-av/av-scenechange), used under the MIT License:
//
// Copyright (c) 2019 Multimedia and Rust
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::cmp;
use std::sync::Arc;

use log::debug;

use super::{fast_idiv, ScaleFunction, SceneChangeDetector, ScenecutResult};
use crate::api::SceneDetectionSpeed;
use crate::frame::{Frame, Plane};
use crate::util::{CastFromPrimitive, Pixel};

/// Experiments have determined this to be an optimal threshold
pub(super) const FAST_THRESHOLD: f64 = 18.0;

impl<T: Pixel> SceneChangeDetector<T> {
  /// The fast algorithm detects fast cuts using a raw difference
  /// in pixel values between the scaled frames.
  pub(super) fn fast_scenecut(
    &mut self, frame1: Arc<Frame<T>>, frame2: Arc<Frame<T>>,
  ) -> ScenecutResult {
    if let Some(scale_func) = &self.scale_func {
      // downscale both frames for faster comparison
      if let Some(frame_buffer) = &mut self.downscaled_frame_buffer {
        frame_buffer.swap(0, 1);
        (scale_func.downscale_in_place)(
          &frame2.planes[0],
          &mut frame_buffer[1],
        );
      } else {
        self.downscaled_frame_buffer = Some([
          (scale_func.downscale)(&frame1.planes[0]),
          (scale_func.downscale)(&frame2.planes[0]),
        ]);
      }

      if let Some(frame_buffer) = &self.downscaled_frame_buffer {
        let &[first, second] = &frame_buffer;
        let delta = self.delta_in_planes(first, second);

        ScenecutResult {
          threshold: self.threshold,
          inter_cost: delta,
          imp_block_cost: delta,
          forward_adjusted_cost: delta,
          backward_adjusted_cost: delta,
        }
      } else {
        unreachable!()
      }
    } else {
      let delta = self.delta_in_planes(&frame1.planes[0], &frame2.planes[0]);

      ScenecutResult {
        threshold: self.threshold,
        inter_cost: delta,
        imp_block_cost: delta,
        backward_adjusted_cost: delta,
        forward_adjusted_cost: delta,
      }
    }
  }

  /// Calculates the average sum of absolute difference (SAD) per pixel
  /// between 2 planes
  fn delta_in_planes(&self, plane1: &Plane<T>, plane2: &Plane<T>) -> f64 {
    let delta = sad_plane(plane1, plane2);

    delta as f64 / self.scaled_pixels as f64
  }
}

/// Scaling factor for frame in scene detection
pub(super) fn detect_scale_factor<T: Pixel>(
  resolution: (usize, usize), speed_mode: SceneDetectionSpeed,
) -> Option<ScaleFunction<T>> {
  let small_edge = cmp::min(resolution.0, resolution.1);
  let scale_func = if speed_mode == SceneDetectionSpeed::Fast {
    match small_edge {
      0..=240 => None,
      241..=480 => Some(ScaleFunction::from_scale::<2>()),
      481..=720 => Some(ScaleFunction::from_scale::<4>()),
      721..=1080 => Some(ScaleFunction::from_scale::<8>()),
      1081..=1600 => Some(ScaleFunction::from_scale::<16>()),
      1601..=usize::MAX => Some(ScaleFunction::from_scale::<32>()),
      _ => None,
    }
  } else {
    None
  };

  if let Some(scale_factor) = scale_func.as_ref().map(|x| x.factor) {
    debug!(
      "Scene detection scale factor {}, [{},{}] -> [{},{}]",
      scale_factor,
      resolution.0,
      resolution.1,
      fast_idiv(resolution.0, scale_factor),
      fast_idiv(resolution.1, scale_factor)
    );
  }

  scale_func
}

/// Calculates the sum of absolute differences between two planes.
fn sad_plane<T: Pixel>(src: &Plane<T>, dst: &Plane<T>) -> u64 {
  assert_eq!(src.cfg.width, dst.cfg.width);
  assert_eq!(src.cfg.height, dst.cfg.height);

  src
    .rows_iter()
    .zip(dst.rows_iter())
    .map(|(src, dst)| {
      src
        .iter()
        .zip(dst.iter())
        .map(|(&p1, &p2)| i32::cast_from(p1).abs_diff(i32::cast_from(p2)))
        .sum::<u32>() as u64
    })
    .sum()
}
