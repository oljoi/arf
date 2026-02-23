//! Run with `cargo run --example subcommands -- build --release -j 8`

use arf::fmt::HelpFormatter;
use arf::{Arg, Parser, SubCommand};

fn parser() -> Parser<'static, 1, 3, 4> {
  Parser::<1, 3, 4>::new("freight", "0.1.0")
    .about("Toy cargo-like CLI")
    .arg(Arg::flag("verbose").short('v').long("verbose"))
    .subcmd(
      SubCommand::<4>::new("build")
        .about("Compile the project")
        .arg(Arg::flag("release").short('r').long("release"))
        .arg(Arg::option("jobs").short('j').long("jobs").meta("N")),
    )
    .subcmd(
      SubCommand::<4>::new("new")
        .about("Create a new project")
        .arg(Arg::positional("path").required())
        .arg(Arg::flag("lib").long("lib")),
    )
    .subcmd(SubCommand::<4>::new("clean").about("Remove build artifacts"))
}

fn main() {
  let parser = parser();
  let argv: Vec<String> = std::env::args().collect();
  let mut buf = [0u8; 2048];
  let mut out = HelpFormatter::new(&mut buf);

  let result = match parser.parse_sub::<2, _>(&argv) {
    Ok(r) => r,
    Err(e) => {
      e.write_error(&mut out).ok();
      eprintln!("{}", out.as_str());
      std::process::exit(2);
    }
  };

  if result.help_requested() {
    if let Some((name, sub)) = result.subcommand() {
      if sub.is_present("help") {
        if let Some(def) = parser.get_subcommand(name) {
          def.format_help(parser.auto_help, parser.auto_version, &mut out).unwrap();
          print!("{}", out.as_str());
          return;
        }
      }
    }
    parser.format_help(&mut out).unwrap();
    print!("{}", out.as_str());
    return;
  }

  let Some((name, sub)) = result.subcommand() else {
    eprintln!("error: expected a subcommand. Try --help");
    std::process::exit(1);
  };

  match name {
    "build" => {
      let mode = if sub.is_present("release") { "release" } else { "dev" };
      print!("Building ({mode})");
      if let Some(j) = sub.value_of("jobs") {
        print!(", {j} jobs");
      }
      println!();
    }
    "new" => {
      let kind = if sub.is_present("lib") { "library" } else { "binary" };
      println!("Creating {kind} project at {}", sub.value_of("path").unwrap());
    }
    "clean" => println!("Cleaning all artifacts"),
    _ => unreachable!(),
  }
}
