extern crate getopts;
extern crate procrs;
use getopts::Options;
use std::env;
use procrs::Proc;

fn main() {
  let prog_opts = match parse_args() {
    Some(t) => { t }
    None => { return; }
  };
  let pid: u32 = prog_opts.query.parse().unwrap();
  let proc_q = Proc::new(pid);
}

struct ProgOpts {
  query: String,
}

fn parse_args() -> Option<ProgOpts> {
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();
  let mut prog_opts = ProgOpts{ query: "".to_string() };

  let mut opts = Options::new();
  opts.optflag("h", "help", "Print help");

  let matches = match opts.parse(&args[1..]) {
    Ok(m) => {m}
    Err(f) => { panic!(f.to_string()) }
  };
  if matches.opt_present("h") {
    print_usage(&program, opts);
    return None;
  }
  if !matches.free.is_empty() {
    prog_opts.query = matches.free[0].clone();

  } else {
    print_usage(&program, opts);
    return None;
  };
  Some(prog_opts)
}

fn print_usage(program: &str, opts: Options) {
  let brief = format!("Usage: {} query [options]", program);
  print!("{}", opts.usage(&brief));
}
