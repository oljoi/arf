//! `no_std`, zero-allocation, const-generic command-line argument parser.
//!
//! Everything lives on the stack. No `alloc`, no `Box`, no `Vec`, no `String`.
//! Capacity is fixed at compile time through const generics, so the parser
//! stays within those bounds.
//!
//! ```
//! use arf::{Arg, Parser};
//!
//! let parser = Parser::<4>::new("example", "0.1.0")
//!   .no_auto_help()
//!   .no_auto_version()
//!   .arg(Arg::flag("verbose").short('v').long("verbose"))
//!   .arg(Arg::option("output").short('o').long("output"));
//!
//! let matches = parser.parse::<4, _>(&["example", "-v", "-o", "out.txt"])?;
//! assert!(matches.is_present("verbose"));
//! assert_eq!(matches.value_of("output"), Some("out.txt"));
//! # Ok::<_, arf::ParseError<'static>>(())
//! ```
//!
//! See the [`Parser`] docs for the const-generic capacity and
//! [`SubCommand`] for subcommand routing.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

mod arg;
mod error;
pub mod fmt;
mod matches;
mod parse;
mod parser;
mod value;

pub use arg::{Arg, ArgKind};
pub use error::{ErrorKind, ParseError, ParseResult};
pub use matches::Matches;
pub use parser::{Parser, SubCommand, SubResult};
pub use value::FromArgValue;
