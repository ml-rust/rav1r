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

use std::sync::Arc;

use super::{SceneChangeDetector, ScenecutResult};
use crate::api::lookahead::estimate_importance_block_difference;
use crate::cpu_features::CpuFeatureLevel;
use crate::dist::get_satd;
use crate::encoder::IMPORTANCE_BLOCK_SIZE;
use crate::frame::{AsRegion, Frame, Plane};
use crate::par::scope;
use crate::partition::BlockSize;
use crate::tiling::Area;
use crate::util::Pixel;

impl<T: Pixel> SceneChangeDetector<T> {
  /// Run a comparison between two frames to determine if they qualify for a
  /// scenecut.
  ///
  /// We gather both intra and inter costs for the frames,
  /// as well as an importance-block-based difference,
  /// and use all three metrics.
  pub(super) fn cost_scenecut(
    &mut self, frame1: Arc<Frame<T>>, frame2: Arc<Frame<T>>,
    input_frameno: usize,
  ) -> ScenecutResult {
    let frame2_inter_ref = Arc::clone(&frame2);
    let frame1_imp_ref = Arc::clone(&frame1);
    let frame2_imp_ref = Arc::clone(&frame2);

    let mut intra_cost = 0.0;
    let mut mv_inter_cost = 0.0;
    let mut imp_block_cost = 0.0;

    scope(|s| {
      s.spawn(|_| {
        let temp_plane =
          self.temp_plane.get_or_insert_with(|| frame2.planes[0].clone());

        let intra_costs = estimate_intra_costs(
          temp_plane,
          &frame2,
          self.bit_depth,
          self.cpu_feature_level,
        );
        if let Some(ref mut intra_cache) = self.intra_costs {
          intra_cache.insert(input_frameno, intra_costs.clone());
        }

        intra_cost = intra_costs.iter().map(|&cost| cost as u64).sum::<u64>()
          as f64
          / intra_costs.len() as f64;
      });
      s.spawn(|_| {
        mv_inter_cost = estimate_inter_costs(
          frame2_inter_ref,
          frame1,
          self.bit_depth,
          self.cpu_feature_level,
        );
      });
      s.spawn(|_| {
        imp_block_cost =
          estimate_importance_block_difference(frame2_imp_ref, frame1_imp_ref);
      });
    });

    // `BIAS` determines how likely we are
    // to choose a keyframe, between 0.0-1.0.
    // Higher values mean we are more likely to choose a keyframe.
    // This value was chosen based on trials using the new
    // adaptive scenecut code.
    const BIAS: f64 = 0.7;
    let threshold = intra_cost * (1.0 - BIAS);

    ScenecutResult {
      inter_cost: mv_inter_cost,
      imp_block_cost,
      threshold,
      backward_adjusted_cost: 0.0,
      forward_adjusted_cost: 0.0,
    }
  }
}

/// Returns the SATD of each block against the constant mid-grey prediction.
fn estimate_intra_costs<T: Pixel>(
  temp_plane: &mut Plane<T>, frame: &Frame<T>, bit_depth: usize,
  cpu_feature_level: CpuFeatureLevel,
) -> Box<[u32]> {
  let plane = &frame.planes[0];
  let plane_after_prediction = temp_plane;

  let bsize = BlockSize::from_width_and_height(
    IMPORTANCE_BLOCK_SIZE,
    IMPORTANCE_BLOCK_SIZE,
  );
  let prediction = T::cast_from(128u32 << (bit_depth - 8));

  let h_in_imp_b = plane.cfg.height / IMPORTANCE_BLOCK_SIZE;
  let w_in_imp_b = plane.cfg.width / IMPORTANCE_BLOCK_SIZE;
  let mut intra_costs = Vec::with_capacity(h_in_imp_b * w_in_imp_b);

  for y in 0..h_in_imp_b {
    for x in 0..w_in_imp_b {
      let area = Area::Rect {
        x: (x * IMPORTANCE_BLOCK_SIZE) as isize,
        y: (y * IMPORTANCE_BLOCK_SIZE) as isize,
        width: IMPORTANCE_BLOCK_SIZE,
        height: IMPORTANCE_BLOCK_SIZE,
      };

      let plane_org = plane.region(area);

      for line in plane_after_prediction
        .region_mut(area)
        .rows_iter_mut()
        .take(bsize.height())
      {
        line[..bsize.width()].fill(prediction);
      }

      let plane_after_prediction_region = plane_after_prediction.region(area);

      let intra_cost = get_satd(
        &plane_org,
        &plane_after_prediction_region,
        bsize.width(),
        bsize.height(),
        bit_depth,
        cpu_feature_level,
      );

      intra_costs.push(intra_cost);
    }
  }

  intra_costs.into_boxed_slice()
}

/// Returns the mean SATD of each block against the collocated reference block.
fn estimate_inter_costs<T: Pixel>(
  frame: Arc<Frame<T>>, ref_frame: Arc<Frame<T>>, bit_depth: usize,
  cpu_feature_level: CpuFeatureLevel,
) -> f64 {
  let plane_org = &frame.planes[0];
  let plane_ref = &ref_frame.planes[0];
  let h_in_imp_b = plane_org.cfg.height / IMPORTANCE_BLOCK_SIZE;
  let w_in_imp_b = plane_org.cfg.width / IMPORTANCE_BLOCK_SIZE;
  let bsize = BlockSize::from_width_and_height(
    IMPORTANCE_BLOCK_SIZE,
    IMPORTANCE_BLOCK_SIZE,
  );

  let mut inter_costs = 0;
  (0..h_in_imp_b).for_each(|y| {
    (0..w_in_imp_b).for_each(|x| {
      let area = Area::Rect {
        x: (x * IMPORTANCE_BLOCK_SIZE) as isize,
        y: (y * IMPORTANCE_BLOCK_SIZE) as isize,
        width: IMPORTANCE_BLOCK_SIZE,
        height: IMPORTANCE_BLOCK_SIZE,
      };

      inter_costs += get_satd(
        &plane_org.region(area),
        &plane_ref.region(area),
        bsize.width(),
        bsize.height(),
        bit_depth,
        cpu_feature_level,
      ) as u64;
    });
  });
  inter_costs as f64 / (w_in_imp_b * h_in_imp_b) as f64
}
