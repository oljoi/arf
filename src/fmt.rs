//! Help formatter outputs help in a predefined pretty format with padding, option names and default values
//!
//! You can use it like this
//!
//! ```
//! use arf::{Arg, Parser, fmt::HelpFormatter};
//!
//! let matches = match parser.parse::<2, _>(&argv) {
//!   Ok(m) => m,
//!   Err(e) => {
//!     let mut buf = [0u8; 256];
//!     let mut fmt = HelpFormatter::new(&mut buf);
//!     e.write_error(&mut fmt).ok();
//!     eprintln!("{}", fmt.as_str());
//!     std::process::exit(2);
//!   }
//! };
//!
//! if matches.help_requested() {
//!   let mut buf = [0u8; 1024];
//!   let mut fmt = HelpFormatter::new(&mut buf);
//!   parser.format_help(&mut fmt).unwrap();
//!   print!("{}", fmt.as_str());
//!   return;
//! }
//!```

use core::fmt;

use crate::arg::Arg;
use crate::parser::SubCommand;

/// `core::fmt::Write` sink over a caller-supplied `&mut [u8]`.
///
/// Writes past the end of the buffer return `core::fmt::Error`.
#[derive(Debug)]
pub struct HelpFormatter<'a> {
  buf: &'a mut [u8],
  pos: usize,
}

impl<'a> HelpFormatter<'a> {
  /// Wraps a provided buffer. The buffer's full length becomes the formatter's capacity
  #[inline]
  pub fn new(buf: &'a mut [u8]) -> Self {
    Self { buf, pos: 0 }
  }

  #[inline]
  pub const fn len(&self) -> usize {
    self.pos
  }

  #[inline]
  pub const fn is_empty(&self) -> bool {
    self.pos == 0
  }

  #[inline]
  pub const fn capacity(&self) -> usize {
    self.buf.len()
  }

  #[inline]
  pub const fn remaining(&self) -> usize {
    self.buf.len() - self.pos
  }

  /// Returns the written bytes as `&str`. Panics if they are not valid UTF-8
  #[inline]
  pub fn as_str(&self) -> &str {
    core::str::from_utf8(&self.buf[..self.pos]).expect("Non-UTF8 data written")
  }

  #[inline]
  pub fn as_bytes(&self) -> &[u8] {
    &self.buf[..self.pos]
  }

  /// Resets the write position to zero. Buffer contents are not cleared
  #[inline]
  pub fn reset(&mut self) {
    self.pos = 0;
  }

  /// Appends one byte. Errors if the buffer is full
  #[inline]
  pub fn write_byte(&mut self, b: u8) -> fmt::Result {
    if self.pos >= self.buf.len() {
      return Err(fmt::Error);
    }
    self.buf[self.pos] = b;
    self.pos += 1;
    Ok(())
  }

  /// Appends `bytes`. Errors if the buffer can't fit them
  #[inline]
  pub fn write_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
    if self.pos + bytes.len() > self.buf.len() {
      return Err(fmt::Error);
    }
    self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
    self.pos += bytes.len();
    Ok(())
  }
}

impl fmt::Write for HelpFormatter<'_> {
  #[inline]
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.write_bytes(s.as_bytes())
  }

  #[inline]
  fn write_char(&mut self, c: char) -> fmt::Result {
    if c.is_ascii() {
      self.write_byte(c as u8)
    } else {
      let mut utf8_buf = [0u8; 4];
      let utf8_str = c.encode_utf8(&mut utf8_buf);
      self.write_str(utf8_str)
    }
  }
}

const SPACES: &str = "                                "; // 32

/// Writes spaces to pad from `current_col` to `target_col`.
///
/// Always emits at least 2 spaces (when already at or past `target_col`),
/// so adjacent text never abuts.
pub fn write_padding<W: fmt::Write>(
  w: &mut W,
  current_col: usize,
  target_col: usize,
) -> fmt::Result {
  let mut n = if current_col >= target_col {
    2
  } else {
    target_col - current_col
  };
  while n > 0 {
    let chunk = n.min(SPACES.len());
    w.write_str(&SPACES[..chunk])?;
    n -= chunk;
  }
  Ok(())
}

/// Display width of `s` in monospace cells.
///
/// Assumes one cell per Unicode scalar value. Does not consult East Asian
/// width data, so CJK and other wide characters undercount.
#[inline]
fn display_width(s: &str) -> usize {
  s.chars().count()
}

/// Joined display width of `values` separated by `|`.
fn possible_joined_width(values: &[&str]) -> usize {
  let sep = values.len().saturating_sub(1);
  let inner: usize = values.iter().map(|v| display_width(v)).sum();
  inner + sep
}

const POSSIBLE_INLINE_BUDGET: usize = 16;

/// Formats a single option/flag line in help output.
pub fn format_option_line<W: fmt::Write>(w: &mut W, arg: &Arg<'_>) -> fmt::Result {
  w.write_str("    ")?;
  let mut col: usize = 4;

  if arg.short != '\0' {
    w.write_str("-")?;
    fmt::Write::write_char(w, arg.short)?;
    col += 2;
    if arg.long.is_some() {
      w.write_str(", ")?;
      col += 2;
    }
  } else {
    w.write_str("    ")?;
    col += 4;
  }

  if let Some(long) = arg.long {
    w.write_str("--")?;
    w.write_str(long)?;
    col += 2 + display_width(long);
  }

  let inline_possible = arg
    .possible
    .filter(|vs| possible_joined_width(vs) <= POSSIBLE_INLINE_BUDGET);

  if arg.kind.is_option() {
    w.write_str(" <")?;
    if let Some(values) = inline_possible {
      for (i, v) in values.iter().enumerate() {
        if i > 0 {
          w.write_str("|")?;
        }
        w.write_str(v)?;
      }
      col += 3 + possible_joined_width(values);
    } else {
      w.write_str(arg.meta)?;
      col += 3 + display_width(arg.meta);
    }
    w.write_str(">")?;
  }

  write_padding(w, col, 28)?;

  if let Some(help) = arg.help {
    w.write_str(help)?;
    w.write_str(" ")?;
  }

  if arg.is_required() {
    w.write_str("[required]")?;
  }
  if let Some(default) = arg.default {
    w.write_str("[default: ")?;
    w.write_str(default)?;
    w.write_str("]")?;
  }
  if inline_possible.is_none() {
    if let Some(values) = arg.possible {
      w.write_str("[possible: ")?;
      for (i, v) in values.iter().enumerate() {
        if i > 0 {
          w.write_str(", ")?;
        }
        w.write_str(v)?;
      }
      w.write_str("]")?;
    }
  }

  w.write_str("\n")
}

/// Writes a simple usage line.
pub fn write_usage<W: fmt::Write>(
  program_name: &str,
  has_options: bool,
  has_args: bool,
  w: &mut W,
) -> fmt::Result {
  w.write_str("Usage: ")?;
  w.write_str(program_name)?;
  if has_options {
    w.write_str(" [OPTIONS]")?;
  }
  if has_args {
    w.write_str(" [ARGS]")?;
  }
  Ok(())
}

/// Writes a version string.
pub fn write_version<W: fmt::Write>(program_name: &str, version: &str, w: &mut W) -> fmt::Result {
  w.write_str(program_name)?;
  w.write_str(" ")?;
  w.write_str(version)
}

/// Format full help output for a parser (shared by Parser and SubCommand).
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_parser_help<const A: usize, W: fmt::Write>(
  w: &mut W,
  name: &str,
  version: &str,
  about: Option<&str>,
  args: &[Arg<'_>],
  auto_help: bool,
  auto_version: bool,
  subs: &[SubCommand<'_, A>],
) -> fmt::Result {
  w.write_str(name)?;
  if !version.is_empty() {
    w.write_str(" ")?;
    w.write_str(version)?;
  }
  w.write_str("\n")?;

  if let Some(about) = about {
    w.write_str(about)?;
    w.write_str("\n")?;
  }

  w.write_str("\nUSAGE:\n")?;
  w.write_str(name)?;

  let has_options = args.iter().any(|a| !a.kind.is_positional()) || auto_help || auto_version;
  if has_options {
    w.write_str(" [OPTIONS]")?;
  }

  let has_subs = !subs.is_empty();
  if has_subs {
    w.write_str(" <SUBCOMMAND>")?;
  }

  let has_positionals = args.iter().any(|a| a.kind.is_positional());
  if has_positionals {
    w.write_str(" [ARGS]")?;
  }
  w.write_str("\n")?;

  if has_options {
    w.write_str("\nOPTIONS:\n")?;
    for arg in args.iter() {
      if !arg.kind.is_positional() {
        format_option_line(w, arg)?;
      }
    }
    if auto_help {
      w.write_str("    -h, --help")?;
      write_padding(w, 14, 28)?;
      w.write_str("Print help information\n")?;
    }
    if auto_version {
      w.write_str("    -V, --version")?;
      write_padding(w, 17, 28)?;
      w.write_str("Print version information\n")?;
    }
  }

  if has_positionals {
    w.write_str("\nARGS:\n")?;
    for arg in args.iter() {
      if arg.kind.is_positional() {
        w.write_str("    <")?;
        w.write_str(arg.name)?;
        w.write_str(">")?;
        let col = 5 + display_width(arg.name) + 1;
        write_padding(w, col, 28)?;
        if let Some(help) = arg.help {
          w.write_str(help)?;
          w.write_str(" ")?;
        }
        if arg.is_required() {
          w.write_str("[required]")?;
        }
        if let Some(values) = arg.possible {
          w.write_str("[possible: ")?;
          for (i, v) in values.iter().enumerate() {
            if i > 0 {
              w.write_str(", ")?;
            }
            w.write_str(v)?;
          }
          w.write_str("]")?;
        }
        w.write_str("\n")?;
      }
    }
  }

  if has_subs {
    w.write_str("\nSUBCOMMANDS:\n")?;
    for sub in subs.iter() {
      if sub.name.is_empty() {
        continue;
      }
      w.write_str("    ")?;
      w.write_str(sub.name)?;
      let col = 4 + display_width(sub.name);
      write_padding(w, col, 28)?;
      if let Some(about) = sub.about {
        w.write_str(about)?;
      }
      w.write_str("\n")?;
    }
  }

  Ok(())
}

impl crate::error::ParseError<'_> {
  /// Writes a CLI-style `error: <message>` line for this error.
  ///
  /// Use this when printing directly to a user; for composing into a larger
  /// message, format with `Display` instead (no prefix).
  pub fn write_error<W: fmt::Write>(self, w: &mut W) -> fmt::Result {
    w.write_str("error: ")?;
    write!(w, "{self}")
  }
}
