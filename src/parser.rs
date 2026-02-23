use crate::arg::Arg;
use crate::error::ParseResult;
use crate::matches::Matches;
use crate::parse;

/// Command-line argument parser with optional subcommand support.
///
/// Const generics fix capacities at compile time, their definition:
/// - `A`: max argument definitions on the parser
/// - `S`: max subcommands (default `0` for flat parsers)
/// - `SA`: max argument definitions per subcommand (default `0`)
pub struct Parser<'a, const A: usize, const S: usize = 0, const SA: usize = 0> {
  /// Program name shown in help and version output.
  pub name: &'a str,
  /// Program version shown in version output.
  pub version: &'a str,
  /// Optional one-line description shown in help.
  pub about: Option<&'a str>,
  /// When `true`, `-h`/`--help` is recognised automatically.
  pub auto_help: bool,
  /// When `true`, `-V`/`--version` is recognised automatically.
  pub auto_version: bool,
  args: [Arg<'a>; A],
  arg_count: usize,
  subs: [SubCommand<'a, SA>; S],
  sub_count: usize,
}

impl<'a, const A: usize, const S: usize, const SA: usize> Parser<'a, A, S, SA> {
  /// Creates a new parser with the given program `name` and `version`.
  ///
  /// Note: bare leading `-` followed by digits (e.g. `-1`) is treated as a
  /// short cluster and will fail with `UnknownArgument`. To pass negative
  /// numbers as positionals, place them after `--`.
  #[inline]
  pub const fn new(name: &'a str, version: &'a str) -> Self {
    const { assert!(A <= 64, "A must be <= 64 (PresenceBits is u64)") }
    const { assert!(SA <= 64, "SA must be <= 64 (PresenceBits is u64)") }
    Self {
      name,
      version,
      about: None,
      auto_help: true,
      auto_version: true,
      args: [Arg::NONE; A],
      arg_count: 0,
      subs: [SubCommand::EMPTY; S],
      sub_count: 0,
    }
  }

  /// Sets the one-line description shown in help output
  #[inline]
  pub const fn about(mut self, text: &'a str) -> Self {
    self.about = Some(text);
    self
  }

  /// Adds an argument definition. Panics if more than `A` are added
  #[inline]
  pub const fn arg(mut self, arg: Arg<'a>) -> Self {
    assert!(self.arg_count < A, "Parser capacity exceeded (increase A)");
    self.args[self.arg_count] = arg;
    self.arg_count += 1;
    self
  }

  /// Disables automatic `-h`/`--help` handling
  #[inline]
  pub const fn no_auto_help(mut self) -> Self {
    self.auto_help = false;
    self
  }

  /// Disables automatic `-V`/`--version` handling
  #[inline]
  pub const fn no_auto_version(mut self) -> Self {
    self.auto_version = false;
    self
  }

  /// Adds a subcommand. Panics if more than `S` are added
  #[inline]
  pub const fn subcmd(mut self, sub: SubCommand<'a, SA>) -> Self {
    assert!(
      self.sub_count < S,
      "Subcommand capacity exceeded (increase S)"
    );
    self.subs[self.sub_count] = sub;
    self.sub_count += 1;
    self
  }

  #[inline]
  pub fn args(&self) -> &[Arg<'a>] {
    &self.args[..self.arg_count]
  }

  #[inline]
  pub const fn arg_count(&self) -> usize {
    self.arg_count
  }

  #[inline]
  pub fn subs(&self) -> &[SubCommand<'a, SA>] {
    &self.subs[..self.sub_count]
  }

  /// Looks up a subcommand definition by name. Handy for routing help
  #[inline]
  pub fn get_subcommand(&self, name: &str) -> Option<&SubCommand<'a, SA>> {
    self.subs[..self.sub_count].iter().find(|s| s.name == name)
  }

  /// Parses `args` (with the program name in slot `0`) and returns [`Matches`].
  ///
  /// `P` is the positional-slot capacity; pick the max count of positionals
  /// the arf will accept.
  ///
  /// For errors see [`crate::error::ErrorKind`]
  pub fn parse<const P: usize, T: AsRef<str>>(
    &self,
    args: &'a [T],
  ) -> ParseResult<'a, Matches<'a, A, P>> {
    let mut matches = Matches::<A, P>::new();
    let arg_defs = &self.args[..self.arg_count];

    for def in arg_defs.iter() {
      matches.add_entry(def.name)?;
    }

    parse::parse_into(
      arg_defs,
      args,
      &mut matches,
      self.auto_help,
      self.auto_version,
    )?;

    let skip = matches.help_requested() || matches.version_requested();
    parse::post_process(arg_defs, &mut matches.entries, &matches.present, skip)?;

    Ok(matches)
  }

  /// Parses `args` with subcommand routing, splitting global args from subcommand-specific args.
  ///
  /// # Errors
  /// See [`crate::error::ErrorKind`] for the full taxonomy.
  pub fn parse_sub<const P: usize, T: AsRef<str>>(
    &self,
    args: &'a [T],
  ) -> ParseResult<'a, SubResult<'a, A, SA, P>> {
    if args.is_empty() || S == 0 {
      let global = self.parse::<P, _>(args)?;
      return Ok(SubResult {
        global,
        sub_name: None,
        sub: None,
      });
    }

    let global_args = &self.args[..self.arg_count];
    let subs = &self.subs[..self.sub_count];
    let mut split: Option<(usize, usize)> = None;
    let mut ti = 1;

    while ti < args.len() {
      let token = args[ti].as_ref();

      if token == "--" {
        break;
      }

      if let Some(after_dashes) = token.strip_prefix("--") {
        let eq_pos = after_dashes.bytes().position(|b| b == b'=');
        let key = match eq_pos {
          Some(pos) => &after_dashes[..pos],
          None => after_dashes,
        };

        if (self.auto_help && key == "help") || (self.auto_version && key == "version") {
          ti += 1;
          continue;
        }

        if let Some(idx) = global_args.iter().position(|a| a.matches_long(key)) {
          if eq_pos.is_none() && global_args[idx].kind.is_option() {
            ti += 2;
          } else {
            ti += 1;
          }
          continue;
        }

        break;
      }

      if token.starts_with('-') && token.len() > 1 {
        if parse::short_cluster_consumes_next(token, global_args, self.auto_help, self.auto_version)
        {
          ti += 2;
        } else {
          ti += 1;
        }
        continue;
      }

      if let Some(sub_idx) = subs.iter().position(|s| s.name == token) {
        split = Some((ti, sub_idx));
      }
      break;
    }

    match split {
      Some((split_idx, sub_idx)) => {
        let global = self.parse::<P, _>(&args[..split_idx])?;
        let sub_def = &subs[sub_idx];
        let sub_matches = self.parse_subcmd_args::<P, T>(sub_def, &args[split_idx..])?;
        Ok(SubResult {
          global,
          sub_name: Some(sub_def.name),
          sub: Some(sub_matches),
        })
      }
      None => {
        let global = self.parse::<P, _>(args)?;
        Ok(SubResult {
          global,
          sub_name: None,
          sub: None,
        })
      }
    }
  }

  fn parse_subcmd_args<const P: usize, T: AsRef<str>>(
    &self,
    sub: &SubCommand<'a, SA>,
    args: &'a [T],
  ) -> ParseResult<'a, Matches<'a, SA, P>> {
    let sub_arg_defs = &sub.args[..sub.arg_count];
    let mut matches = Matches::<SA, P>::new();

    for def in sub_arg_defs.iter() {
      matches.add_entry(def.name)?;
    }

    parse::parse_into(
      sub_arg_defs,
      args,
      &mut matches,
      self.auto_help,
      self.auto_version,
    )?;

    let skip = matches.help_requested() || matches.version_requested();
    parse::post_process(sub_arg_defs, &mut matches.entries, &matches.present, skip)?;

    Ok(matches)
  }

  /// Renders full help output for this parser into `w`.
  ///
  /// # Errors
  /// Propagates `core::fmt::Error` from the writer (e.g. fixed-buffer overflow).
  pub fn format_help<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
    crate::fmt::format_parser_help(
      w,
      self.name,
      self.version,
      self.about,
      &self.args[..self.arg_count],
      self.auto_help,
      self.auto_version,
      &self.subs[..self.sub_count],
    )
  }
}

impl<const A: usize, const S: usize, const SA: usize> core::fmt::Debug for Parser<'_, A, S, SA> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("Parser { name: \"")?;
    f.write_str(self.name)?;
    f.write_str("\", args: [")?;
    for (i, arg) in self.args().iter().enumerate() {
      if i > 0 {
        f.write_str(", ")?;
      }
      core::fmt::Display::fmt(arg, f)?;
    }
    f.write_str("] }")
  }
}

/// A subcommand definition with its own fixed-size argument array.
#[derive(Debug, Clone, Copy)]
pub struct SubCommand<'a, const A: usize> {
  /// Name matched against bare tokens in the input.
  pub name: &'a str,
  /// One-line description shown in the parent's help output.
  pub about: Option<&'a str>,
  pub(crate) args: [Arg<'a>; A],
  pub(crate) arg_count: usize,
}

impl<'a, const A: usize> SubCommand<'a, A> {
  pub(crate) const EMPTY: Self = Self {
    name: "",
    about: None,
    args: [Arg::NONE; A],
    arg_count: 0,
  };

  /// Starts a subcommand named `name`.
  #[inline]
  pub const fn new(name: &'a str) -> Self {
    Self {
      name,
      about: None,
      args: [Arg::NONE; A],
      arg_count: 0,
    }
  }

  /// Sets the one-line description shown in the parent's help output.
  #[inline]
  pub const fn about(mut self, text: &'a str) -> Self {
    self.about = Some(text);
    self
  }

  /// Adds an argument to this subcommand. Panics if more than `A` are added.
  #[inline]
  pub const fn arg(mut self, arg: Arg<'a>) -> Self {
    assert!(
      self.arg_count < A,
      "SubCommand capacity exceeded (increase A)"
    );
    self.args[self.arg_count] = arg;
    self.arg_count += 1;
    self
  }

  /// Renders help for this subcommand into `w`.
  ///
  /// `auto_help` / `auto_version` reflect the parent parser's settings so
  /// the rendered output matches what the parser actually accepts.
  ///
  /// Propagates `core::fmt::Error` from the writer.
  pub fn format_help<W: core::fmt::Write>(
    &self,
    auto_help: bool,
    auto_version: bool,
    w: &mut W,
  ) -> core::fmt::Result {
    crate::fmt::format_parser_help::<A, W>(
      w,
      self.name,
      "",
      self.about,
      &self.args[..self.arg_count],
      auto_help,
      auto_version,
      &[],
    )
  }
}

/// Result of [`Parser::parse_sub`]: global matches plus optional subcommand matches.
///
/// `A` is the parser's argument capacity, `SA` the subcommand's, `P` the
/// positional capacity. If no subcommand was matched, `sub_name` and `sub`
/// are both `None`.
pub struct SubResult<'a, const A: usize, const SA: usize, const P: usize> {
  /// Matches for arguments defined on the top-level parser.
  pub global: Matches<'a, A, P>,
  /// Which subcommand was invoked, if any.
  pub sub_name: Option<&'a str>,
  /// Matches for the subcommand's own arguments.
  pub sub: Option<Matches<'a, SA, P>>,
}

impl<'a, const A: usize, const SA: usize, const P: usize> SubResult<'a, A, SA, P> {
  /// Returns `Some((name, matches))` if a subcommand was matched.
  #[inline]
  pub fn subcommand(&self) -> Option<(&'a str, &Matches<'a, SA, P>)> {
    match (self.sub_name, self.sub.as_ref()) {
      (Some(name), Some(matches)) => Some((name, matches)),
      _ => None,
    }
  }

  #[inline]
  pub fn has_subcommand(&self) -> bool {
    self.sub_name.is_some()
  }

  /// Returns `true` if help was requested at the global or subcommand level.
  #[inline]
  pub fn help_requested(&self) -> bool {
    self.global.help_requested() || self.sub.as_ref().is_some_and(|s| s.help_requested())
  }

  /// Returns `true` if version was requested at the global or subcommand level.
  #[inline]
  pub fn version_requested(&self) -> bool {
    self.global.version_requested() || self.sub.as_ref().is_some_and(|s| s.version_requested())
  }
}

impl<const A: usize, const SA: usize, const P: usize> core::fmt::Debug for SubResult<'_, A, SA, P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("SubResult { sub_name: ")?;
    match self.sub_name {
      Some(n) => {
        f.write_str("\"")?;
        f.write_str(n)?;
        f.write_str("\"")?;
      }
      None => {
        f.write_str("None")?;
      }
    }
    f.write_str(" }")
  }
}
