extern crate procrs;
extern crate argparse;
#[macro_use]
extern crate prettytable;
use prettytable::Table;
use prettytable::format::FormatBuilder;
use std::collections::HashMap;
use procrs::prc::*;
use argparse::{ArgumentParser, StoreTrue, Store};

fn main() {
  let opts = parse_args();
  match opts {
    ProgOpts{ tree: false, query: q, .. } => {
      let mut table = Table::init(
        ProcIter::new_query(q).unwrap()
          .map(|p_r|
            p_r.map(|p|
              row![p.stat.pid, p.stat.ppid, p.stat.comm, p.cmdline.join(" ")]
            )
          ).collect::<Result<Vec<_>, String>>()
          .unwrap()
      );
      let format = FormatBuilder::new()
        .column_separator(' ')
        .build();
      table.set_titles(row!["Pid", "Ppid", "Comm", "Cmd"]);
      table.set_format(format);
      table.printstd();
    },

    ProgOpts{ tree: true, query: q, .. } => {
      let proc_map: HashMap<_, _> =
        ProcIter::new().unwrap()
        .map(|p_r|
          p_r.map(|p|
            (p.stat.pid, p)
          )
        ).collect::<Result<_, String>>()
        .unwrap();

      let mut child_procs = HashMap::new();
      let mut proc_list = Vec::new();

      for (pid, proc_struct) in &proc_map {
        proc_list.push(pid);
        let ppid = proc_struct.stat.ppid;
        child_procs.entry(ppid)
          .or_insert(Vec::new())
          .push(proc_struct);
      }
      proc_list.sort();
      let pid = match q {
        ProcQuery::PidQuery(p) => p,
        _ => 1
      };
      let mut pid_procs = Vec::new();

      let start_procs = match proc_map.get(&pid) {
        Some(proc_struct) => {
          pid_procs.push(proc_struct);
          &pid_procs
        },
        None => child_procs.get(&pid).expect("Invalid pid")
      };

      print_tree(&child_procs, start_procs, "".to_string());
    }
  }
}

fn print_tree(child_procs: &HashMap<TaskId, Vec<&Proc>>,
              level_procs: &Vec<&Proc>, prefix: String) {
  let mut proc_list = level_procs.to_vec();
  proc_list.sort();
  for proc_struct in proc_list {
    let pid = &proc_struct.stat.pid;

    println!("{}{}", prefix, proc_struct.stat.comm);
    let child_list = child_procs.get(pid);
    if let Some(v) = child_list {
      print_tree(child_procs, v, format!("{}{}", "  ", prefix));
    }
  }
}

struct ProgOpts {
  query: ProcQuery,
  tree: bool,
  verbose: bool
}

fn parse_args() -> ProgOpts {
  let mut opts = ProgOpts {
    query: ProcQuery::NoneQuery,
    tree: false,
    verbose: false
  };

  {
    let mut ap = ArgumentParser::new();
    ap.set_description("Query linux processes");
    ap.refer(&mut opts.tree)
      .add_option(&["-t", "--tree"], StoreTrue, "Display process tree");
    ap.refer(&mut opts.verbose)
      .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
    ap.refer(&mut opts.query)
      .add_argument("query", Store, "Optional query to search by, pid or string");
    ap.parse_args_or_exit();
  }

  opts
}
