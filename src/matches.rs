use crate::error::{ErrorKind, ParseError};
use crate::value::FromArgValue;

/// Bitset over up to 64 entries, used to track which user arguments have
/// been observed in the input.
#[derive(Clone, Copy)]
pub(crate) struct PresenceBits(u64);

impl PresenceBits {
  #[inline]
  pub(crate) const fn new() -> Self {
    Self(0)
  }

  #[inline]
  pub(crate) fn set(&mut self, idx: usize) {
    self.0 |= 1u64 << idx;
  }

  #[inline]
  pub(crate) fn get(&self, idx: usize) -> bool {
    self.0 & (1u64 << idx) != 0
  }

  #[inline]
  pub(crate) fn count_ones(&self) -> usize {
    self.0.count_ones() as usize
  }
}

impl core::fmt::Debug for PresenceBits {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "PresenceBits({:016x})", self.0)
  }
}

/// Result of a successful parse: argument values plus positionals.
///
/// `A` is the number of user argument definitions (matches the parser's `A`).
/// `P` is the positional-slot capacity supplied at the parse call site.
///
/// Auto-help and auto-version live in dedicated booleans — they never consume
/// room from the `A` budget. Look them up by the reserved names `"help"`
/// and `"version"`.
pub struct Matches<'a, const A: usize, const P: usize> {
  pub(crate) names: [&'a str; A],
  pub(crate) entries: [Option<&'a str>; A],
  pub(crate) present: PresenceBits,
  pub(crate) entry_count: usize,
  pub(crate) help: bool,
  pub(crate) version: bool,
  pub(crate) positionals: [Option<&'a str>; P],
  pub(crate) pos_count: usize,
}

impl<'a, const A: usize, const P: usize> Matches<'a, A, P> {
  #[inline]
  pub(crate) fn new() -> Self {
    const { assert!(A <= 64, "A must be <= 64 (PresenceBits is u64)") }
    Self {
      names: [""; A],
      entries: [None; A],
      present: PresenceBits::new(),
      entry_count: 0,
      help: false,
      version: false,
      positionals: [None; P],
      pos_count: 0,
    }
  }

  #[inline]
  pub(crate) fn add_entry(&mut self, name: &'a str) -> Result<usize, ParseError<'a>> {
    if self.entry_count >= A {
      return Err(ParseError::new(ErrorKind::TooManyArguments).with_name(name));
    }
    let idx = self.entry_count;
    self.names[idx] = name;
    self.entry_count += 1;
    Ok(idx)
  }

  #[inline]
  pub(crate) fn set_present(&mut self, idx: usize) {
    self.present.set(idx);
  }

  #[inline]
  pub(crate) fn get_present(&self, idx: usize) -> bool {
    self.present.get(idx)
  }

  #[inline]
  pub(crate) fn add_positional(&mut self, value: &'a str) -> Result<(), ParseError<'a>> {
    if self.pos_count >= P {
      return Err(ParseError::new(ErrorKind::TooManyPositionals).with_token(value));
    }
    self.positionals[self.pos_count] = Some(value);
    self.pos_count += 1;
    Ok(())
  }

  #[inline]
  fn find_index(&self, name: &str) -> Option<usize> {
    self.names[..self.entry_count]
      .iter()
      .position(|&n| n == name)
  }

  /// Returns `true` if `name` was present in the parsed input.
  pub fn is_present(&self, name: &str) -> bool {
    match name {
      "help" if self.help => true,
      "version" if self.version => true,
      _ => self.find_index(name).is_some_and(|i| self.get_present(i)),
    }
  }

  #[inline]
  pub fn help_requested(&self) -> bool {
    self.help
  }

  #[inline]
  pub fn version_requested(&self) -> bool {
    self.version
  }

  /// Returns the string value of an option (or its configured default).
  ///
  /// For auto-help / auto-version, returns `Some("true")` when requested.
  pub fn value_of(&self, name: &str) -> Option<&'a str> {
    match name {
      "help" if self.help => Some("true"),
      "version" if self.version => Some("true"),
      _ => self.find_index(name).and_then(|i| self.entries[i as usize]),
    }
  }

  /// Returns the string value, or `default` if not present.
  #[inline]
  pub fn value_of_or(&self, name: &str, default: &'a str) -> &'a str {
    self.value_of(name).unwrap_or(default)
  }

  /// Parses the value of `name` into `T` via [`FromArgValue`].
  ///
  /// Returns `Ok(None)` if absent with no default, `Ok(Some(_))` on success,
  /// or `Err` if the value exists but [`FromArgValue::from_arg_value`] rejects it.
  ///
  /// # Errors
  /// [`ErrorKind::InvalidValue`] if the value is present but rejected.
  pub fn value_of_parsed<T: FromArgValue>(
    &self,
    name: &'a str,
  ) -> Result<Option<T>, ParseError<'a>> {
    match self.value_of(name) {
      Some(s) => match T::from_arg_value(s) {
        Some(val) => Ok(Some(val)),
        None => Err(
          ParseError::new(ErrorKind::InvalidValue)
            .with_token(s)
            .with_name(name),
        ),
      },
      None => Ok(None),
    }
  }

  /// Parses the value of `name` into `T`, returning `default` if absent.
  ///
  /// # Errors
  /// [`ErrorKind::InvalidValue`] if the value is present but rejected by [`FromArgValue`].
  pub fn value_of_parsed_or<T: FromArgValue>(
    &self,
    name: &'a str,
    default: T,
  ) -> Result<T, ParseError<'a>> {
    Ok(self.value_of_parsed::<T>(name)?.unwrap_or(default))
  }

  /// Returns the positional value at `index`, or `None` if out of range.
  #[inline]
  pub fn positional(&self, index: usize) -> Option<&'a str> {
    if index < self.pos_count as usize {
      self.positionals[index]
    } else {
      None
    }
  }

  /// Parses the positional at `index` into `T`.
  ///
  /// # Errors
  /// [`ErrorKind::InvalidValue`] if the value exists but [`FromArgValue`] rejects it.
  pub fn positional_parsed<T: FromArgValue>(
    &self,
    index: usize,
  ) -> Result<Option<T>, ParseError<'a>> {
    match self.positional(index) {
      Some(s) => match T::from_arg_value(s) {
        Some(val) => Ok(Some(val)),
        None => Err(ParseError::new(ErrorKind::InvalidValue).with_token(s)),
      },
      None => Ok(None),
    }
  }

  #[inline]
  pub const fn positional_count(&self) -> usize {
    self.pos_count as usize
  }

  #[inline]
  pub fn positionals(&self) -> &[Option<&'a str>] {
    &self.positionals[..self.pos_count as usize]
  }

  /// Total number of named arguments observed in the input
  /// (user-defined plus any auto-help / auto-version hits).
  pub fn present_count(&self) -> usize {
    self.present.count_ones() + usize::from(self.help) + usize::from(self.version)
  }

  /// Returns `true` if no named arguments or positionals were captured.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.present_count() == 0 && self.pos_count == 0
  }
}

impl<const A: usize, const P: usize> core::fmt::Debug for Matches<'_, A, P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("Matches {\n")?;
    f.write_str("  entries: [\n")?;
    for i in 0..self.entry_count {
      let idx = i as usize;
      f.write_str("    ")?;
      f.write_str(self.names[idx])?;
      f.write_str(": present=")?;
      if self.get_present(i) {
        f.write_str("true")?;
      } else {
        f.write_str("false")?;
      }
      if let Some(v) = self.entries[idx] {
        f.write_str(", value=\"")?;
        f.write_str(v)?;
        f.write_str("\"")?;
      }
      f.write_str("\n")?;
    }
    f.write_str("  ],\n")?;
    if self.help {
      f.write_str("  help: requested\n")?;
    }
    if self.version {
      f.write_str("  version: requested\n")?;
    }
    f.write_str("  positionals: [")?;
    for j in 0..self.pos_count {
      if j > 0 {
        f.write_str(", ")?;
      }
      if let Some(v) = self.positionals[j as usize] {
        f.write_str("\"")?;
        f.write_str(v)?;
        f.write_str("\"")?;
      }
    }
    f.write_str("],\n")?;
    f.write_str("}")
  }
}
