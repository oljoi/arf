/// Kind of argument an [`Arg`] represents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
  /// Boolean switch, takes no value (`-v`, `--verbose`)
  Flag,
  /// Named argument that takes a value (`-o file`, `--output=file`)
  Option,
  /// Unnamed value at a fixed slot
  Positional,
}

impl ArgKind {
  /// Lowercase label: `"flag"`, `"option"`, or `"positional"`
  #[inline]
  pub const fn as_str(&self) -> &'static str {
    match self {
      ArgKind::Flag => "flag",
      ArgKind::Option => "option",
      ArgKind::Positional => "positional",
    }
  }

  #[inline]
  pub const fn is_flag(&self) -> bool {
    matches!(self, ArgKind::Flag)
  }

  #[inline]
  pub const fn is_option(&self) -> bool {
    matches!(self, ArgKind::Option)
  }

  #[inline]
  pub const fn is_positional(&self) -> bool {
    matches!(self, ArgKind::Positional)
  }
}

const FLAG_REQUIRED: u8 = 1 << 0;
const FLAG_ALLOW_DUPLICATES: u8 = 1 << 1;

/// Definition of a single command-line argument.
///
/// Construct one of [`flag`](Arg::flag), [`option`](Arg::option), or
/// [`positional`](Arg::positional), then chain builder methods. All builders
/// are `const`, so an `Arg` can be declared in a `const` context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arg<'a> {
  pub name: &'a str,
  /// Short flag character. `'\0'` means no short form
  pub short: char,
  pub kind: ArgKind,
  pub long: Option<&'a str>,
  pub help: Option<&'a str>,
  /// Placeholder shown in help output for options (`--out <FILE>`). Defaults to `"VALUE"`
  pub meta: &'a str,
  /// Default value applied when the option is absent
  pub default: Option<&'a str>,
  /// Allowed values. Parsing fails with [`ErrorKind::InvalidChoice`](crate::ErrorKind::InvalidChoice) if the input is not in the slice
  pub possible: Option<&'a [&'a str]>,
  flags: u8,
}

impl<'a> Arg<'a> {
  pub(crate) const NONE: Self = Self {
    name: "",
    short: '\0',
    kind: ArgKind::Flag,
    flags: 0,
    long: None,
    help: None,
    meta: "VALUE",
    default: None,
    possible: None,
  };

  /// Create a boolean switch
  #[inline]
  pub const fn flag(name: &'a str) -> Self {
    Self {
      name,
      short: '\0',
      kind: ArgKind::Flag,
      flags: 0,
      long: None,
      help: None,
      meta: "VALUE",
      default: None,
      possible: None,
    }
  }

  /// Create a value-taking option
  #[inline]
  pub const fn option(name: &'a str) -> Self {
    Self {
      name,
      short: '\0',
      kind: ArgKind::Option,
      flags: 0,
      long: None,
      help: None,
      meta: "VALUE",
      default: None,
      possible: None,
    }
  }

  /// Create a positional argument
  #[inline]
  pub const fn positional(name: &'a str) -> Self {
    Self {
      name,
      short: '\0',
      kind: ArgKind::Positional,
      flags: 0,
      long: None,
      help: None,
      meta: "VALUE",
      default: None,
      possible: None,
    }
  }

  /// Sets the short form. Positionals cannot have one
  #[inline]
  pub const fn short(mut self, c: char) -> Self {
    assert!(
      !self.kind.is_positional(),
      "positional arguments cannot have short flags"
    );
    self.short = c;
    self
  }

  /// Sets the long form without the `--` prefix. Positionals cannot have one
  #[inline]
  pub const fn long(mut self, s: &'a str) -> Self {
    assert!(
      !self.kind.is_positional(),
      "positional arguments cannot have long flags"
    );
    self.long = Some(s);
    self
  }

  /// Sets the help text shown in generated help output
  #[inline]
  pub const fn help(mut self, text: &'a str) -> Self {
    self.help = Some(text);
    self
  }

  /// Sets the value placeholder shown in help output (`--out <NAME>`). Defaults to `"VALUE"`
  #[inline]
  pub const fn meta(mut self, name: &'a str) -> Self {
    self.meta = name;
    self
  }

  /// Marks the argument as required; parsing fails if absent
  #[inline]
  pub const fn required(mut self) -> Self {
    self.flags |= FLAG_REQUIRED;
    self
  }

  /// Sets a default value used when the option is absent
  #[inline]
  pub const fn default(mut self, val: &'a str) -> Self {
    self.default = Some(val);
    self
  }

  /// Restricts the value to one of `values`. Flags cannot have one
  #[inline]
  pub const fn possible(mut self, values: &'a [&'a str]) -> Self {
    assert!(
      !self.kind.is_flag(),
      "flag arguments cannot have possible values"
    );
    self.possible = Some(values);
    self
  }

  /// Permits the argument to appear more than once. Last value is used
  #[inline]
  pub const fn allow_duplicates(mut self) -> Self {
    self.flags |= FLAG_ALLOW_DUPLICATES;
    self
  }

  #[inline]
  pub const fn is_required(&self) -> bool {
    self.flags & FLAG_REQUIRED != 0
  }

  #[inline]
  pub const fn allows_duplicates(&self) -> bool {
    self.flags & FLAG_ALLOW_DUPLICATES != 0
  }

  #[inline]
  pub const fn is_empty(&self) -> bool {
    self.name.is_empty()
  }

  pub(crate) const fn matches_short(&self, c: char) -> bool {
    if self.short == '\0' || self.kind.is_positional() {
      return false;
    }
    self.short == c
  }

  pub(crate) fn matches_long(&self, s: &str) -> bool {
    if self.kind.is_positional() {
      return false;
    }
    match self.long {
      Some(l) => l == s,
      None => false,
    }
  }
}

impl core::fmt::Display for Arg<'_> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str(self.kind.as_str())?;
    f.write_str("(")?;

    let mut sep = false;

    if self.short != '\0' {
      f.write_str("-")?;
      core::fmt::Write::write_char(f, self.short)?;
      sep = true;
    }

    if let Some(long) = self.long {
      if sep {
        f.write_str(", ")?;
      }
      f.write_str("--")?;
      f.write_str(long)?;
      sep = true;
    }

    if self.kind.is_option() {
      if sep {
        f.write_str(" ")?;
      }
      f.write_str("<")?;
      f.write_str(self.meta)?;
      f.write_str(">")?;
    }

    if self.kind.is_positional() {
      f.write_str(self.name)?;
    }

    f.write_str(")")
  }
}
