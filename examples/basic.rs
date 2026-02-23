use arf::fmt::HelpFormatter;
use arf::{Arg, Parser};

fn main() {
  let argv: Vec<String> = std::env::args().collect();

  let parser = Parser::<8>::new("arf", "0.6.7")
    .about("A sample application")
    .arg(
      Arg::flag("verbose")
        .short('v')
        .long("verbose")
        .help("Enable verbose output"),
    )
    .arg(
      Arg::option("output")
        .short('o')
        .long("output")
        .help("Output file path")
        .required(),
    )
    .arg(
      Arg::option("number")
        .short('n')
        .long("number")
        .help("Number to parse")
        .possible(&["1-99"]),
    )
    .arg(Arg::option("юникод").short('ю').long("юни").help("юникод"))
    .arg(
      Arg::positional("input")
        .help("Input file to process")
        .possible(&["test"]),
    );

  let matches = match parser.parse::<2, _>(&argv) {
    Ok(m) => m,
    Err(e) => {
      let mut buf = [0u8; 256];
      let mut fmt = HelpFormatter::new(&mut buf);
      e.write_error(&mut fmt).ok();
      eprintln!("{}", fmt.as_str());
      std::process::exit(2);
    }
  };

  if matches.help_requested() {
    let mut buf = [0u8; 1024];
    let mut fmt = HelpFormatter::new(&mut buf);
    parser.format_help(&mut fmt).unwrap();
    print!("{}", fmt.as_str());
    return;
  }

  if matches.version_requested() {
    println!("{} {}", parser.name, parser.version);
    return;
  }

  println!("verbose = {}", matches.is_present("verbose"));
  println!("output  = {}", matches.value_of_or("output", ""));
  println!("input   = {}", matches.value_of_or("input", "<none>"));
  println!("matches = {:#?}", matches);
}
