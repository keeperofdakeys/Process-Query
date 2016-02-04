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
    ProgOpts { query: Some(q), tree: false, .. } => {
      let pid = q.parse().unwrap();
      let proc_struct = Proc::new(pid);
      println!("{:?}", proc_struct);
    },

    ProgOpts { query: q, tree: true, .. } => {
      let proc_map = get_proc_map().unwrap();

      let mut child_procs = HashMap::new();
      let mut proc_list = Vec::new();

      for (pid, proc_struct) in &proc_map {
        proc_list.push(pid);
        let ppid = proc_struct.status.ppid;
        child_procs.entry(ppid)
          .or_insert(Vec::new())
          .push(proc_struct);
      }
      proc_list.sort();
      let pid = q.and_then(|p| p.parse().ok()).unwrap_or(1);;
      let mut pid_procs = Vec::new();

      let start_procs = match proc_map.get(&pid) {
        Some(proc_struct) => {
          pid_procs.push(proc_struct);
          &pid_procs
        },
        None => child_procs.get(&pid).unwrap()
      };

      print_tree(&child_procs, start_procs, "".to_string());
    }
    _ => {
      println!("{}", "Bad arguments");
      // print_usage();
      return;
    }
  }
}

fn print_tree(child_procs: &HashMap<TaskId, Vec<&Proc>>,
              level_procs: &Vec<&Proc>, prefix: String) {
  let mut proc_list = level_procs.to_vec();
  proc_list.sort();
  for proc_struct in proc_list {
    let pid = &proc_struct.status.pid;

    println!("{}{}", prefix, proc_struct.status.name);
    // println!("{}{:#?}", prefix, proc_struct);
    let child_list = child_procs.get(pid);
    if let Some(v) = child_list {
      print_tree(child_procs, v, format!("{}{}", "  ", prefix));
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
