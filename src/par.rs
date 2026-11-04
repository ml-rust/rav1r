// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! Parallel iteration and thread pools.
//!
//! With the `threading` feature this module re-exports the items of `rayon`
//! that the encoder uses. Without it, the same items run on the calling
//! thread.

#[cfg(feature = "threading")]
pub use rayon::{iter, ThreadPool, ThreadPoolBuilder};

#[cfg(not(feature = "threading"))]
pub use sequential::*;

#[cfg(not(feature = "threading"))]
mod sequential {
  use std::convert::Infallible;

  pub mod iter {
    /// Sequential stand-in for `rayon::iter::IntoParallelIterator`.
    pub trait IntoParallelIterator {
      type Item: Send;
      type Iter: Iterator<Item = Self::Item>;

      fn into_par_iter(self) -> Self::Iter;
    }

    impl<I: IntoIterator> IntoParallelIterator for I
    where
      I::Item: Send,
    {
      type Item = I::Item;
      type Iter = I::IntoIter;

      #[inline(always)]
      fn into_par_iter(self) -> Self::Iter {
        self.into_iter()
      }
    }

    /// Sequential stand-in for `rayon::iter::IntoParallelRefMutIterator`.
    pub trait IntoParallelRefMutIterator<'data> {
      type Item: Send + 'data;
      type Iter: Iterator<Item = Self::Item>;

      fn par_iter_mut(&'data mut self) -> Self::Iter;
    }

    impl<'data, T: Send + 'data> IntoParallelRefMutIterator<'data> for [T] {
      type Item = &'data mut T;
      type Iter = std::slice::IterMut<'data, T>;

      #[inline(always)]
      fn par_iter_mut(&'data mut self) -> Self::Iter {
        self.iter_mut()
      }
    }
  }

  /// Sequential stand-in for `rayon::ThreadPoolBuilder`.
  #[derive(Default)]
  pub struct ThreadPoolBuilder;

  impl ThreadPoolBuilder {
    #[inline(always)]
    pub fn new() -> Self {
      ThreadPoolBuilder
    }

    #[inline(always)]
    pub fn num_threads(self, _num_threads: usize) -> Self {
      self
    }

    #[inline(always)]
    pub fn build(self) -> Result<ThreadPool, Infallible> {
      Ok(ThreadPool)
    }
  }

  /// Sequential stand-in for `rayon::ThreadPool`.
  #[derive(Debug)]
  pub struct ThreadPool;

  impl ThreadPool {
    /// Runs `op` on the calling thread.
    #[inline(always)]
    pub fn install<OP, R>(&self, op: OP) -> R
    where
      OP: FnOnce() -> R + Send,
      R: Send,
    {
      op()
    }
  }
}
