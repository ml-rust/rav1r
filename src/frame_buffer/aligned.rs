// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

//! A heap slice with a minimum alignment.

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::slice;

/// A fixed-length heap slice aligned to at least `ALIGN` bytes.
///
/// `ALIGN` is a power of two. The element type is `Copy`, so dropping the
/// slice releases the allocation and runs no element destructor.
pub struct AlignedSlice<T: Copy, const ALIGN: usize> {
  ptr: NonNull<T>,
  len: usize,
  owned: PhantomData<T>,
}

// SAFETY: `AlignedSlice` owns its elements and shares no state between
// values, so it has the thread-safety of the elements.
unsafe impl<T: Copy + Send, const ALIGN: usize> Send
  for AlignedSlice<T, ALIGN>
{
}
// SAFETY: A shared `AlignedSlice` only hands out shared element references.
unsafe impl<T: Copy + Sync, const ALIGN: usize> Sync
  for AlignedSlice<T, ALIGN>
{
}

impl<T: Copy, const ALIGN: usize> AlignedSlice<T, ALIGN> {
  /// Returns the allocation layout for `len` elements.
  ///
  /// # Panics
  ///
  /// - If the size in bytes does not fit in `isize`
  fn layout(len: usize) -> Layout {
    const { assert!(ALIGN.is_power_of_two()) };
    let align = ALIGN.max(align_of::<T>());
    let layout = len
      .checked_mul(size_of::<T>())
      .and_then(|size| Layout::from_size_align(size, align).ok());
    match layout {
      Some(layout) => layout,
      None => panic!(
        "cannot allocate {len} elements of {} bytes: request a smaller plane",
        size_of::<T>()
      ),
    }
  }

  /// Returns uninitialized storage for `len` elements.
  ///
  /// The caller writes all `len` elements before it builds a slice over them.
  fn allocate(len: usize) -> NonNull<T> {
    let layout = Self::layout(len);
    if layout.size() == 0 {
      // SAFETY: `align` is a power of two, so the address is not null. The
      // pointer is never read, because the slice over it has no bytes.
      return unsafe {
        NonNull::new_unchecked(ptr::without_provenance_mut(layout.align()))
      };
    }
    // SAFETY: `layout` has a non-zero size.
    let raw = unsafe { alloc(layout) };
    match NonNull::new(raw.cast::<T>()) {
      Some(ptr) => ptr,
      None => handle_alloc_error(layout),
    }
  }

  /// Allocates `len` elements, each set to `value`.
  ///
  /// # Panics
  ///
  /// - If the size in bytes does not fit in `isize`
  pub fn from_elem(len: usize, value: T) -> Self {
    let ptr = Self::allocate(len);
    for i in 0..len {
      // SAFETY: `ptr` holds `len` elements, and `i` is below `len`.
      unsafe { ptr.as_ptr().add(i).write(value) };
    }
    Self { ptr, len, owned: PhantomData }
  }

  /// Allocates a copy of `data`.
  ///
  /// # Panics
  ///
  /// - If the size in bytes does not fit in `isize`
  pub fn from_slice(data: &[T]) -> Self {
    let ptr = Self::allocate(data.len());
    // SAFETY: `ptr` holds `data.len()` elements in a new allocation, so the
    // two ranges do not overlap.
    unsafe {
      ptr::copy_nonoverlapping(data.as_ptr(), ptr.as_ptr(), data.len());
    }
    Self { ptr, len: data.len(), owned: PhantomData }
  }
}

impl<T: Copy, const ALIGN: usize> Drop for AlignedSlice<T, ALIGN> {
  fn drop(&mut self) {
    let layout = Self::layout(self.len);
    if layout.size() != 0 {
      // SAFETY: `allocate` returned this pointer from `alloc` with this layout.
      unsafe { dealloc(self.ptr.as_ptr().cast(), layout) };
    }
  }
}

impl<T: Copy, const ALIGN: usize> Deref for AlignedSlice<T, ALIGN> {
  type Target = [T];

  #[inline]
  fn deref(&self) -> &[T] {
    // SAFETY: `ptr` is aligned and holds `len` initialized elements.
    unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
  }
}

impl<T: Copy, const ALIGN: usize> DerefMut for AlignedSlice<T, ALIGN> {
  #[inline]
  fn deref_mut(&mut self) -> &mut [T] {
    // SAFETY: `ptr` is aligned and holds `len` initialized elements, and
    // `self` is borrowed mutably.
    unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
  }
}

impl<T: Copy, const ALIGN: usize> Clone for AlignedSlice<T, ALIGN> {
  fn clone(&self) -> Self {
    Self::from_slice(self)
  }
}

impl<T: Copy + Debug, const ALIGN: usize> Debug for AlignedSlice<T, ALIGN> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    (**self).fmt(f)
  }
}

impl<T: Copy + PartialEq, const ALIGN: usize> PartialEq
  for AlignedSlice<T, ALIGN>
{
  fn eq(&self, other: &Self) -> bool {
    **self == **other
  }
}

impl<T: Copy + Eq, const ALIGN: usize> Eq for AlignedSlice<T, ALIGN> {}

#[cfg(feature = "serialize")]
impl<T: Copy + serde::Serialize, const ALIGN: usize> serde::Serialize
  for AlignedSlice<T, ALIGN>
{
  fn serialize<S: serde::Serializer>(
    &self, serializer: S,
  ) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&**self, serializer)
  }
}

#[cfg(feature = "serialize")]
impl<'de, T: Copy + serde::Deserialize<'de>, const ALIGN: usize>
  serde::Deserialize<'de> for AlignedSlice<T, ALIGN>
{
  fn deserialize<D: serde::Deserializer<'de>>(
    deserializer: D,
  ) -> Result<Self, D::Error> {
    let data = <Vec<T> as serde::Deserialize<'de>>::deserialize(deserializer)?;
    Ok(Self::from_slice(&data))
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn check_aligned<T: Copy + PartialEq + Debug, const ALIGN: usize>(value: T) {
    for len in [0, 1, 2, 3, 63, 64, 65, 127, 128, 129, 4096, 4097] {
      let data = AlignedSlice::<T, ALIGN>::from_elem(len, value);
      assert_eq!(data.as_ptr() as usize % ALIGN, 0, "from_elem len {len}");
      assert_eq!(data.len(), len);
      assert!(data.iter().all(|&v| v == value));

      let copy = data.clone();
      assert_eq!(copy.as_ptr() as usize % ALIGN, 0, "clone len {len}");
      assert_eq!(copy, data);
    }
  }

  #[test]
  fn aligned_to_requested_boundary() {
    check_aligned::<u8, 64>(0x80);
    check_aligned::<u16, 64>(0x200);
    check_aligned::<i16, 64>(-1);
    check_aligned::<u8, 128>(0x80);
    check_aligned::<u16, 128>(0x200);
    check_aligned::<i16, 128>(-1);
    check_aligned::<u64, 8>(7);
    check_aligned::<u64, 1>(7);
  }

  #[test]
  fn from_slice_copies_every_element() {
    let source: Vec<u16> = (0..1000).collect();
    let data = AlignedSlice::<u16, 64>::from_slice(&source);
    assert_eq!(data.as_ptr() as usize % 64, 0);
    assert_eq!(&*data, &source[..]);
  }

  #[test]
  fn mutation_stays_in_the_owner() {
    let mut data = AlignedSlice::<u8, 64>::from_elem(16, 1);
    let copy = data.clone();
    data[3] = 9;
    assert_eq!(data[3], 9);
    assert_eq!(copy[3], 1);
    assert_ne!(data, copy);
  }

  #[test]
  #[should_panic(expected = "cannot allocate")]
  fn oversized_request_panics() {
    let _ = AlignedSlice::<u16, 64>::from_elem(usize::MAX, 0);
  }
}
