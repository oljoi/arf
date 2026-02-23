/// Parses a value out of a raw argument string.
///
/// Implement this for custom types to use [`Matches::value_of_parsed`](crate::Matches::value_of_parsed)
/// and [`Matches::positional_parsed`](crate::Matches::positional_parsed). The crate provides
/// impls for `bool` and all built-in integer types.
///
/// Returning `None` surfaces as [`ErrorKind::InvalidValue`](crate::ErrorKind::InvalidValue).
pub trait FromArgValue: Sized {
  /// Parses `s`, returning `None` if it cannot be interpreted as `Self`.
  fn from_arg_value(s: &str) -> Option<Self>;
}

impl FromArgValue for bool {
  fn from_arg_value(s: &str) -> Option<Self> {
    match s.as_bytes() {
      b"true" | b"True" | b"TRUE" => Some(true),
      b"yes" | b"Yes" | b"YES" => Some(true),
      b"on" | b"On" | b"ON" => Some(true),
      b"1" => Some(true),
      b"t" | b"T" => Some(true),
      b"y" | b"Y" => Some(true),
      b"false" | b"False" | b"FALSE" => Some(false),
      b"no" | b"No" | b"NO" => Some(false),
      b"off" | b"Off" | b"OFF" => Some(false),
      b"0" => Some(false),
      b"f" | b"F" => Some(false),
      b"n" | b"N" => Some(false),
      _ => None,
    }
  }
}

macro_rules! impl_unsigned {
  ($($ty:ty),+ $(,)?) => {
    $(
      impl FromArgValue for $ty {
        fn from_arg_value(s: &str) -> Option<Self> {
          let bytes = s.as_bytes();
          if bytes.is_empty() { return None; }
          let mut result: $ty = 0;
          for &b in bytes {
            if !b.is_ascii_digit() { return None; }
            result = result.checked_mul(10)?;
            result = result.checked_add((b - b'0') as $ty)?;
          }
          Some(result)
        }
      }
    )+
  };
}

macro_rules! impl_signed {
  ($(($s:ty, $u:ty)),+ $(,)?) => {
    $(
      impl FromArgValue for $s {
        fn from_arg_value(s: &str) -> Option<Self> {
          let bytes = s.as_bytes();
          if bytes.is_empty() { return None; }
          let (neg, mag_bytes) = if bytes[0] == b'-' {
            if bytes.len() == 1 { return None; }
            (true, &bytes[1..])
          } else {
            (false, bytes)
          };
          let mut mag: $u = 0;
          for &b in mag_bytes {
            if !b.is_ascii_digit() { return None; }
            mag = mag.checked_mul(10)?;
            mag = mag.checked_add((b - b'0') as $u)?;
          }
          if neg {
            let max_mag = (<$s>::MIN as $u);
            if mag > max_mag { return None; }
            Some(mag.wrapping_neg() as $s)
          } else {
            if mag > (<$s>::MAX as $u) { return None; }
            Some(mag as $s)
          }
        }
      }
    )+
  };
}

impl_unsigned!(u8, u16, u32, u64, u128, usize);
impl_signed!(
  (i8, u8),
  (i16, u16),
  (i32, u32),
  (i64, u64),
  (i128, u128),
  (isize, usize),
);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn u8_from() {
    assert_eq!(u8::from_arg_value("0"), Some(0));
    assert_eq!(u8::from_arg_value("000"), Some(0));
    assert_eq!(u8::from_arg_value("255"), Some(255));
    assert_eq!(u8::from_arg_value("256"), None);
    assert_eq!(u8::from_arg_value(""), None);
    assert_eq!(u8::from_arg_value("abc"), None);
    assert_eq!(u8::from_arg_value("-1"), None);
    assert_eq!(u8::from_arg_value("007"), Some(7));
  }

  #[test]
  fn u16_from() {
    assert_eq!(u16::from_arg_value("8080"), Some(8080));
    assert_eq!(u16::from_arg_value("65535"), Some(65535));
    assert_eq!(u16::from_arg_value("65536"), None);
  }

  #[test]
  fn i8_from() {
    assert_eq!(i8::from_arg_value("-39"), Some(-39));
    assert_eq!(i8::from_arg_value("62"), Some(62));
    assert_eq!(i8::from_arg_value("0"), Some(0));
    assert_eq!(i8::from_arg_value("-0"), Some(0));
    assert_eq!(i8::from_arg_value("127"), Some(127));
    assert_eq!(i8::from_arg_value("-128"), Some(-128));
    assert_eq!(i8::from_arg_value("128"), None);
    assert_eq!(i8::from_arg_value("-129"), None);
    assert_eq!(i8::from_arg_value("-"), None);
  }

  #[test]
  fn i32_from() {
    assert_eq!(i32::from_arg_value("2147483647"), Some(i32::MAX));
    assert_eq!(i32::from_arg_value("-2147483648"), Some(i32::MIN));
    assert_eq!(i32::from_arg_value("2147483648"), None);
  }
}
