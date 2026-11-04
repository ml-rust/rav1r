// Copyright (c) 2026, The rav1r contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

/// Implements `FromStr`, `Display` and `variants()` for a fieldless enum.
///
/// List every variant. A variant takes an optional alias after `=`.
/// `FromStr` matches a variant name or alias without regard to case.
/// `Display` writes the variant name.
///
/// ```ignore
/// arg_enum!(Foo { A, B = "bar" });
/// ```
macro_rules! arg_enum {
  ($name:ident { $($variant:ident $(= $alias:literal)?),+ $(,)? }) => {
    impl ::std::str::FromStr for $name {
      type Err = String;

      fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        $(
          if s.eq_ignore_ascii_case(stringify!($variant))
            $(|| s.eq_ignore_ascii_case($alias))?
          {
            return Ok($name::$variant);
          }
        )+
        Err(format!("valid values: {}", Self::variants().join(" ,")))
      }
    }

    impl ::std::fmt::Display for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
          $($name::$variant => f.write_str(stringify!($variant)),)+
        }
      }
    }

    impl $name {
      /// Returns the names that `FromStr` accepts for this enum.
      #[allow(dead_code)]
      pub fn variants() -> &'static [&'static str] {
        &[$(stringify!($variant) $(, $alias)?),+]
      }
    }
  };
}
