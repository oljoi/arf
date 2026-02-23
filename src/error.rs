use core::fmt;

/// Why parsing failed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
  /// `--arg` doesn't match any defined argument
  UnknownArgument,
  /// `--arg` was given with no value after it
  MissingValue,
  /// A `.required()` argument wasn't provided
  MissingRequired,
  /// Same argument appeared twice without `.allow_duplicates()`
  DuplicateArgument,
  /// More positional values than `P` slots in `Matches<A, P>`
  TooManyPositionals,
  /// More argument definitions than `A` slots
  TooManyArguments,
  /// `FromArgValue` returned `None` for the given input
  InvalidValue,
  /// Value was not in the [`possible`](crate::Arg::possible) set
  InvalidChoice,
  /// Bare `-` with nothing after it
  EmptyFlag,
}

impl ErrorKind {
  /// Short lowercase label for this error kind
  #[inline]
  pub const fn as_str(&self) -> &'static str {
    match self {
      ErrorKind::UnknownArgument => "unknown argument",
      ErrorKind::MissingValue => "missing value",
      ErrorKind::MissingRequired => "missing required argument",
      ErrorKind::DuplicateArgument => "duplicate argument",
      ErrorKind::TooManyPositionals => "too many positional arguments",
      ErrorKind::TooManyArguments => "too many argument definitions",
      ErrorKind::InvalidValue => "invalid value",
      ErrorKind::InvalidChoice => "invalid choice",
      ErrorKind::EmptyFlag => "empty flag",
    }
  }
}

impl fmt::Display for ErrorKind {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

/// A parse failure with optional context borrowed from the input.
///
/// Build with [`ParseError::new`] then attach context via the
/// `with_*` methods. Borrows from the argv slice; lifetime `'a`
/// matches the input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParseError<'a> {
  pub kind: ErrorKind,
  /// Raw token from the input that triggered the error
  pub token: Option<&'a str>,
  /// Argument name (the lookup key from [`Arg`](crate::Arg))
  pub name: Option<&'a str>,
  /// Byte offset within the token
  pub offset: Option<usize>,
  /// Length of the offending span
  pub len: Option<usize>,
  /// Allowed values when [`kind`](Self::kind) is [`InvalidChoice`](ErrorKind::InvalidChoice)
  pub possible: Option<&'a [&'a str]>,
}

impl<'a> ParseError<'a> {
  /// Creates a new error with `kind` and no context attached
  #[inline]
  pub const fn new(kind: ErrorKind) -> Self {
    Self {
      kind,
      token: None,
      name: None,
      offset: None,
      len: None,
      possible: None,
    }
  }

  /// Attaches the raw input token that caused the error
  #[inline]
  pub const fn with_token(mut self, token: &'a str) -> Self {
    self.token = Some(token);
    self
  }

  /// Attaches the argument name the error relates to
  #[inline]
  pub const fn with_name(mut self, name: &'a str) -> Self {
    self.name = Some(name);
    self
  }

  /// Attaches a byte offset into the offending token
  #[inline]
  pub const fn with_offset(mut self, offset: usize) -> Self {
    self.offset = Some(offset);
    self
  }

  /// Attaches the length of the offending span
  #[inline]
  pub const fn with_len(mut self, len: usize) -> Self {
    self.len = Some(len);
    self
  }

  /// Attaches the allowed-values list (for [`ErrorKind::InvalidChoice`])
  #[inline]
  pub const fn with_possible(mut self, values: &'a [&'a str]) -> Self {
    self.possible = Some(values);
    self
  }
}

impl<'a> fmt::Display for ParseError<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.kind {
      ErrorKind::MissingValue => {
        let arg = self.token.or(self.name).unwrap_or("unknown");
        write!(f, "argument '{arg}' requires a value")
      }
      ErrorKind::UnknownArgument => {
        let arg = self.token.or(self.name).unwrap_or("unknown");
        write!(f, "unknown argument '{arg}'")
      }
      ErrorKind::MissingRequired => {
        let arg = self.name.or(self.token).unwrap_or("unknown");
        write!(f, "required argument '{arg}' was not provided")
      }
      ErrorKind::DuplicateArgument => {
        let arg = self.token.or(self.name).unwrap_or("unknown");
        write!(f, "argument '{arg}' was provided more than once")
      }
      ErrorKind::InvalidValue => {
        let val = self.token.unwrap_or("unknown");
        let arg = self.name.unwrap_or("unknown");
        write!(f, "invalid value '{val}' for argument '{arg}'")
      }
      ErrorKind::InvalidChoice => {
        let val = self.token.unwrap_or("unknown");
        let arg = self.name.unwrap_or("unknown");
        write!(f, "invalid value '{val}' for argument '{arg}'")?;
        if let Some(values) = self.possible {
          f.write_str(", expected one of: ")?;
          for (i, v) in values.iter().enumerate() {
            if i > 0 {
              f.write_str(", ")?;
            }
            f.write_str(v)?;
          }
        }
        Ok(())
      }
      ErrorKind::TooManyPositionals => {
        let arg = self.token.unwrap_or("value");
        write!(f, "unexpected positional argument '{arg}'")
      }
      ErrorKind::TooManyArguments => f.write_str("too many arguments provided"),
      ErrorKind::EmptyFlag => f.write_str("empty flag '-'"),
    }
  }
}

#[cfg(feature = "std")]
impl<'a> std::error::Error for ParseError<'a> {}

/// Alias for `Result<T, ParseError<'a>>`
pub type ParseResult<'a, T> = Result<T, ParseError<'a>>;
