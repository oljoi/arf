use crate::arg::{Arg, ArgKind};
use crate::error::{ErrorKind, ParseError};
use crate::matches::{Matches, PresenceBits};

/// Process a long token (`--foo` or `--foo=bar`).
///
/// `help` / `version` slots: `Some(&mut bool)` enables interception,
/// `None` leaves the token to fall through to user-defined args.
///
/// Returns `true` if the next token was consumed as a value.
pub(crate) fn process_long<'a>(
  token: &'a str,
  next_token: Option<&'a str>,
  arg_defs: &[Arg<'a>],
  entries: &mut [Option<&'a str>],
  present: &mut PresenceBits,
  help: Option<&mut bool>,
  version: Option<&mut bool>,
) -> Result<bool, ParseError<'a>> {
  let after_dashes = &token[2..];

  let (key, inline_value) = match after_dashes.bytes().position(|b| b == b'=') {
    Some(eq_pos) => (&after_dashes[..eq_pos], Some(&after_dashes[eq_pos + 1..])),
    None => (after_dashes, None),
  };

  if key == "help" {
    if let Some(slot) = help {
      if *slot {
        return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name("help"));
      }
      *slot = true;
      return Ok(false);
    }
  }
  if key == "version" {
    if let Some(slot) = version {
      if *slot {
        return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name("version"));
      }
      *slot = true;
      return Ok(false);
    }
  }

  let arg_idx = arg_defs
    .iter()
    .position(|a| a.matches_long(key))
    .ok_or_else(|| ParseError::new(ErrorKind::UnknownArgument).with_token(token))?;

  let arg = &arg_defs[arg_idx];

  if present.get(arg_idx) && !arg.allows_duplicates() {
    return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name(arg.name));
  }

  match arg.kind {
    ArgKind::Flag => {
      present.set(arg_idx);
      entries[arg_idx] = Some("true");
      Ok(false)
    }
    ArgKind::Option => match inline_value {
      Some(val) => {
        present.set(arg_idx);
        entries[arg_idx] = Some(val);
        Ok(false)
      }
      None => match next_token {
        Some(next) => {
          present.set(arg_idx);
          entries[arg_idx] = Some(next);
          Ok(true)
        }
        None => Err(
          ParseError::new(ErrorKind::MissingValue)
            .with_token(token)
            .with_name(arg.name),
        ),
      },
    },
    ArgKind::Positional => unreachable!("cannot return positional"),
  }
}

/// Process a short cluster like `-vd` or `-ofile.txt`.
///
/// `help` / `version` slots behave as in [`process_long`].
///
/// Returns `true` if the next token was consumed as a value.
pub(crate) fn process_short<'a>(
  token: &'a str,
  next_token: Option<&'a str>,
  arg_defs: &[Arg<'a>],
  entries: &mut [Option<&'a str>],
  present: &mut PresenceBits,
  mut help: Option<&mut bool>,
  mut version: Option<&mut bool>,
) -> Result<bool, ParseError<'a>> {
  let mut iter = token.char_indices();
  iter.next();

  while let Some((i, c)) = iter.next() {
    let user_idx = arg_defs.iter().position(|a| a.matches_short(c));

    if user_idx.is_none() {
      if c == 'h' {
        if let Some(slot) = help.as_deref_mut() {
          if *slot {
            return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name("help"));
          }
          *slot = true;
          continue;
        }
      }
      if c == 'V' {
        if let Some(slot) = version.as_deref_mut() {
          if *slot {
            return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name("version"));
          }
          *slot = true;
          continue;
        }
      }
      return Err(ParseError::new(ErrorKind::UnknownArgument).with_token(token));
    }

    let arg_idx = user_idx.unwrap();
    let arg = &arg_defs[arg_idx];

    if present.get(arg_idx) && !arg.allows_duplicates() {
      return Err(ParseError::new(ErrorKind::DuplicateArgument).with_name(arg.name));
    }

    match arg.kind {
      ArgKind::Flag => {
        present.set(arg_idx);
        entries[arg_idx] = Some("true");
      }
      ArgKind::Option => {
        let remaining_start = i + c.len_utf8();
        if remaining_start < token.len() {
          present.set(arg_idx);
          entries[arg_idx] = Some(&token[remaining_start..]);
          return Ok(false);
        }
        match next_token {
          Some(next) => {
            present.set(arg_idx);
            entries[arg_idx] = Some(next);
            return Ok(true);
          }
          None => {
            return Err(
              ParseError::new(ErrorKind::MissingValue)
                .with_token(token)
                .with_name(arg.name),
            );
          }
        }
      }
      ArgKind::Positional => unreachable!("cannot return positional"),
    }
  }

  Ok(false)
}

/// Apply defaults, validate required args and possible-value sets.
pub(crate) fn post_process<'a>(
  arg_defs: &[Arg<'a>],
  entries: &mut [Option<&'a str>],
  present: &PresenceBits,
  skip_required_check: bool,
) -> Result<(), ParseError<'a>> {
  for (i, arg) in arg_defs.iter().enumerate() {
    if arg.is_empty() {
      continue;
    }
    if !present.get(i) {
      if let Some(default) = arg.default {
        entries[i] = Some(default);
      }
      if !skip_required_check && arg.is_required() && arg.default.is_none() {
        return Err(ParseError::new(ErrorKind::MissingRequired).with_name(arg.name));
      }
    }
    if let (Some(values), Some(val)) = (arg.possible, entries[i]) {
      if !values.iter().any(|v| *v == val) {
        return Err(
          ParseError::new(ErrorKind::InvalidChoice)
            .with_token(val)
            .with_name(arg.name)
            .with_possible(values),
        );
      }
    }
  }
  Ok(())
}

/// Check if a short cluster's last char is an option that consumes the next token.
pub(crate) fn short_cluster_consumes_next(
  token: &str,
  global_args: &[Arg<'_>],
  auto_help: bool,
  auto_version: bool,
) -> bool {
  let mut iter = token.char_indices();
  iter.next(); // skip '-'

  while let Some((i, c)) = iter.next() {
    if (auto_help && c == 'h') || (auto_version && c == 'V') {
      continue;
    }

    if let Some(idx) = global_args.iter().position(|a| a.matches_short(c)) {
      if global_args[idx].kind.is_option() {
        return i + c.len_utf8() >= token.len();
      }
    } else {
      return false;
    }
  }

  false
}

/// Shared parsing core: walks `args` (skipping `args[0]`) and routes tokens
/// into `matches`. Defaults and required-checks are handled by [`post_process`].
pub(crate) fn parse_into<'a, const A: usize, const P: usize, T: AsRef<str>>(
  arg_defs: &[Arg<'a>],
  args: &'a [T],
  matches: &mut Matches<'a, A, P>,
  auto_help: bool,
  auto_version: bool,
) -> Result<(), ParseError<'a>> {
  if args.len() <= 1 {
    return Ok(());
  }

  let mut pos_arg_indices = [0usize; A];
  let mut pos_arg_count: usize = 0;
  for (i, def) in arg_defs.iter().enumerate() {
    if def.kind.is_positional() && pos_arg_count < A {
      pos_arg_indices[pos_arg_count] = i;
      pos_arg_count += 1;
    }
  }

  let mut positional_only = false;
  let mut current_pos_def: usize = 0;
  let mut ti = 1;

  while ti < args.len() {
    let token = args[ti].as_ref();

    if positional_only {
      add_positional(
        matches,
        &pos_arg_indices,
        pos_arg_count,
        &mut current_pos_def,
        token,
      )?;
      ti += 1;
      continue;
    }

    if token == "--" {
      positional_only = true;
      ti += 1;
      continue;
    }

    if token.starts_with("--") {
      let next_token = args.get(ti + 1).map(AsRef::as_ref);
      let help_slot = if auto_help {
        Some(&mut matches.help)
      } else {
        None
      };
      let version_slot = if auto_version {
        Some(&mut matches.version)
      } else {
        None
      };
      let consumed_next = process_long(
        token,
        next_token,
        arg_defs,
        &mut matches.entries,
        &mut matches.present,
        help_slot,
        version_slot,
      )?;
      ti += if consumed_next { 2 } else { 1 };
      continue;
    }

    if token.starts_with('-') {
      if token.len() == 1 {
        return Err(ParseError::new(ErrorKind::EmptyFlag).with_token(token));
      }
      let next_token = args.get(ti + 1).map(AsRef::as_ref);
      let help_slot = if auto_help {
        Some(&mut matches.help)
      } else {
        None
      };
      let version_slot = if auto_version {
        Some(&mut matches.version)
      } else {
        None
      };
      let consumed_next = process_short(
        token,
        next_token,
        arg_defs,
        &mut matches.entries,
        &mut matches.present,
        help_slot,
        version_slot,
      )?;
      ti += if consumed_next { 2 } else { 1 };
      continue;
    }

    add_positional(
      matches,
      &pos_arg_indices,
      pos_arg_count,
      &mut current_pos_def,
      token,
    )?;
    ti += 1;
  }

  Ok(())
}

fn add_positional<'a, const A: usize, const P: usize>(
  matches: &mut Matches<'a, A, P>,
  pos_arg_indices: &[usize; A],
  pos_arg_count: usize,
  current_pos_def: &mut usize,
  value: &'a str,
) -> Result<(), ParseError<'a>> {
  matches.add_positional(value)?;
  if *current_pos_def < pos_arg_count {
    let arg_idx = pos_arg_indices[*current_pos_def];
    matches.set_present(arg_idx);
    matches.entries[arg_idx] = Some(value);
    *current_pos_def += 1;
  }
  Ok(())
}
