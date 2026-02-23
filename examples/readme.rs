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
