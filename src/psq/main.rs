extern crate getopts;
extern crate procrs;
use getopts::Options;
use std::env;
use std::collections::HashMap;
use procrs::*;

fn main() {
  let prog_opts = match parse_args() {
    Some(t) => { t }
    None => { return; }
  };
  match prog_opts {
    ProgOpts { query: Some(q), tree: t, .. } => {
      let pid: u32 = q.parse().unwrap();
      let proc_struct = Proc::new(pid);
      println!("{:?}", proc_struct);
    },
    ProgOpts { tree: true, .. } => {
      let proc_map = get_proc_map().unwrap();

      let mut child_procs = HashMap::new();
      let mut proc_list = Vec::new();

      for (pid, proc_struct) in &proc_map {
        proc_list.push(pid);
        child_procs.insert(proc_struct.status.ppid, proc_struct);
      }
      proc_list.sort();
      for pid in proc_list {
        let proc_struct = match proc_map.get(&pid) {
          Some(p) => p,
          _ => continue
        };
        println!("{:?}", proc_struct);
      }
    }
    _ => {
      println!("{}", "Bad arguments");
      // print_usage();
      return;
    }
  }
}

struct ProgOpts {
  query: Option<String>,
  tree: bool
}

fn parse_args() -> Option<ProgOpts> {
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();
  let mut prog_opts = ProgOpts{
    query: None,
    tree: false
  };

  let mut opts = Options::new();
  opts.optflag("h", "help", "Print help");
  opts.optflag("t", "tree", "Print tree");

  let matches = match opts.parse(&args[1..]) {
    Ok(m) => {m}
    Err(f) => { panic!(f.to_string()) }
  };
  if matches.opt_present("h") {
    print_usage(&program, opts);
    return None;
  }
  if matches.opt_present("t") {
    prog_opts.tree = true;
  }
  if !matches.free.is_empty() {
    prog_opts.query = Some(matches.free[0].clone());
  };
  Some(prog_opts)
}

fn print_usage(program: &str, opts: Options) {
  let brief = format!("Usage: {} query [options]", program);
  print!("{}", opts.usage(&brief));
}
