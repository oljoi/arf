use arf::fmt::HelpFormatter;
use arf::{Arg, FromArgValue, Parser};

#[derive(Debug)]
enum LogLevel {
  Error,
  Warn,
  Info,
  Debug,
}

impl FromArgValue for LogLevel {
  fn from_arg_value(s: &str) -> Option<Self> {
    match s {
      "error" => Some(Self::Error),
      "warn" => Some(Self::Warn),
      "info" => Some(Self::Info),
      "debug" => Some(Self::Debug),
      _ => None,
    }
  }
}

fn main() {
  let parser = Parser::<6>::new("complete", env!("CARGO_PKG_VERSION"))
    .about("Kitchen-sink example for arf")
    .arg(
      Arg::flag("verbose")
        .short('v')
        .long("verbose")
        .allow_duplicates(),
    )
    .arg(Arg::flag("force").short('f').long("force"))
    .arg(
      Arg::option("output")
        .short('o')
        .long("output")
        .meta("FILE")
        .required(),
    )
    .arg(
      Arg::option("jobs")
        .short('j')
        .long("jobs")
        .meta("N")
        .default("4"),
    )
    .arg(
      Arg::option("log")
        .short('l')
        .long("log")
        .default("info")
        .possible(&["error", "warn", "info", "debug"]),
    )
    .arg(Arg::positional("input"));

  let argv: Vec<String> = std::env::args().collect();
  let mut buf = [0u8; 2048];
  let mut out = HelpFormatter::new(&mut buf);

  let matches = match parser.parse::<4, _>(&argv) {
    Ok(m) => m,
    Err(e) => {
      e.write_error(&mut out).ok();
      eprintln!("{}", out.as_str());
      std::process::exit(2);
    }
  };

  if matches.help_requested() {
    parser.format_help(&mut out).unwrap();
    print!("{}", out.as_str());
    return;
  }
  if matches.version_requested() {
    println!("{} {}", parser.name, parser.version);
    return;
  }

  let output = matches.value_of("output").unwrap();
  let jobs: u32 = matches.value_of_parsed_or("jobs", 4).unwrap();
  let log: LogLevel = matches.value_of_parsed_or("log", LogLevel::Info).unwrap();

  println!("output  {output}");
  println!("jobs    {jobs}");
  println!("log     {log:?}");
  println!("verbose {}", matches.is_present("verbose"));
  println!("force   {}", matches.is_present("force"));
  if let Some(input) = matches.positional(0) {
    println!("input   {input}");
  }
}
