# arf

`no_std`, zero-allocation, const-generic command-line argument parser for
embedded and size-constrained binaries.

- No heap, so everything is on the stack.
- Fully safe rust and `no_std` by default. Optional `std` feature for `std::error::Error`.
- Capacity is fixed at compile time through const generics.
- Const builders, so entire parsers can be declared in a `const` context.
- Zero dependencies.
- Crate size is about ~30 KiB in release. (based on [rosetta-rs/argparse-rosetta-rs](https://github.com/rosetta-rs/argparse-rosetta-rs))

## Install

```toml
[dependencies]
arf = "0.1"
```

## Example

```rust
use arf::{fmt::HelpFormatter, Arg, ParseResult, Parser};

fn main() -> ParseResult<'static, ()> {
  let parser = Parser::<4>::new("example", "0.1.0")
    .about("Process a file")
    .arg(Arg::flag("verbose").short('v').long("verbose"))
    .arg(Arg::option("output").short('o').long("output").required())
    .arg(Arg::positional("input"));

  let matches = parser.parse::<2, _>(&["example", "--help"])?;

  if matches.help_requested() {
    let mut buf = [0u8; 1024];
    let mut fmt = HelpFormatter::new(&mut buf);
    parser.format_help(&mut fmt).unwrap();
    print!("{}", fmt.as_str());
    return Ok(());
  }

  Ok(())
}
```

then `./example` will produce something like that:
```bash
example 0.1.0
Process a file

USAGE:
example [OPTIONS] [ARGS]

OPTIONS:
    -v, --verbose
    -o, --output <VALUE>    [required]
    -h, --help              Print help information
    -V, --version           Print version information

ARGS:
    <input>
```

## Capacity model

Capacities are const generics, so the parser carries no heap state:

| Generic | Where | Meaning |
|--------:|-------|---------|
| `A`     | `Parser<A>` / `SubCommand<A>` | Max number of arg definitions |
| `S`     | `Parser<A, S, SA>`            | Max number of subcommands |
| `SA`    | `Parser<A, S, SA>`            | Max args per subcommand |
| `P`     | `parser.parse::<P, _>(...)`   | Max positional values captured |

Exceeding `A` or `S` at build time panics. Exceeding `P` at parse time
returns [`ErrorKind::TooManyPositionals`].

## Subcommands

```rust
use arf::{Arg, Parser, SubCommand};

let parser = Parser::<2, 2, 2>::new("git", "0.1.0")
  .arg(Arg::flag("verbose").short('v'))
  .subcmd(
    SubCommand::<2>::new("clone")
      .about("Clone a repository")
      .arg(Arg::positional("url").required()),
  )
  .subcmd(SubCommand::<2>::new("status").about("Show status"));

let result = parser.parse_sub::<2, _>(&["git", "-v", "clone", "https://x"])?;
assert!(result.global.is_present("verbose"));
if let Some((name, sub)) = result.subcommand() {
  assert_eq!(name, "clone");
  assert_eq!(sub.value_of("url"), Some("https://x"));
}
# Ok::<_, arf::ParseError<'static>>(())
```

Use [`Parser::get_subcommand`] to look up a subcommand definition by name
when routing help.

## Custom value types

Implement [`FromArgValue`] for any type used with `value_of_parsed`:

```rust
use arf::FromArgValue;

enum LogLevel { Error, Warn, Info, Debug }

impl FromArgValue for LogLevel {
  fn from_arg_value(s: &str) -> Option<Self> {
    match s {
      "error" => Some(Self::Error),
      "warn"  => Some(Self::Warn),
      "info"  => Some(Self::Info),
      "debug" => Some(Self::Debug),
      _ => None,
    }
  }
}
```

`bool` and every built-in integer (`u8`…`u128`, `i8`…`i128`, `usize`, `isize`)
are implemented for you.

## Errors

Parsing returns [`ParseError`], a `Copy` borrow of the offending token.
[`ParseError::write_error`] prints a CLI-style `error: <message>` line;
the `Display` impl emits the message without a prefix so callers can compose.

## Examples

- `examples/basic.rs` — minimal flag/option/positional setup
- `examples/complete.rs` — every feature in one file with a custom value type
- `examples/subcommands.rs` — cargo-like subcommand routing

Run with `cargo run --example <name>`.

## License

MIT
