extern crate ghqrs;
extern crate clap;

use clap::{Arg, App, SubCommand};

fn main() {
  let matches = App::new("ghqrs")
    .about("Remote management")
    .version("0.0.1")
    .author("Yusuke Sasaki <yusuke.sasaki.nuem@gmail.com>")
    .subcommand(SubCommand::with_name("list")
      .about("List locally cloned repositories")
      .arg(Arg::with_name("exact")
        .short("e")
        .long("exact")
        .help("Perform an exact match"))
      .arg(Arg::with_name("fullpath")
        .short("p")
        .long("full-path")
        .help("print full paths"))
      .arg(Arg::with_name("unique")
        .long("unique")
        .help("Print unique subpaths")))
    .subcommand(SubCommand::with_name("root")
      .about("Show repositories's root")
      .arg(Arg::with_name("all")
        .long("all")
        .help("Show all roots")))
    .get_matches();

  let exitcode = match matches.subcommand_name() {
    Some(ref s) => {
      let ref matches = matches.subcommand_matches(s).unwrap();
      match *s {
        "list" => {
          let exact = matches.is_present("exact");
          let fullpath = matches.is_present("fullpath");
          let unique = matches.is_present("unique");
          ghqrs::command_list(exact, fullpath, unique)
        }
        "root" => {
          let all = matches.is_present("all");
          ghqrs::command_root(all)
        }
        _ => panic!("invalid subcommand: {}", s),
      }
    }
    None => panic!("Invalid subcommand"),
  };
  std::process::exit(exitcode);
}
