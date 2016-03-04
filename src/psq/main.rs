extern crate procrs;
extern crate argparse;
#[macro_use]
extern crate prettytable;
use prettytable::Table;
use prettytable::format::FormatBuilder;
use std::collections::HashMap;
use std::iter::repeat;
use procrs::pid::*;
use procrs::TaskId;
use argparse::{ArgumentParser, StoreTrue, Store};

fn main() {
    let opts = parse_args();
    let (query, long, perf, verbose, tree) =
        (opts.query, opts.long, opts.perf, opts.verbose, opts.tree);

    let mut pids = PidIter::new_query(query).unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap();

    let mut name_indent = HashMap::new();

    if verbose {
        for pid in pids {
            println!("{:?}", pid);
        }
        return
    }

    if opts.tree {
        pids = treeify_names(pids, &mut name_indent);
    } else {
        pids.sort_by(|p1, p2| p1.stat.pid.cmp(&p2.stat.pid));
    };
    // Assume hertz is 100.
    // TODO: Look this up via syscall (no /proc value for it)
    let hertz = 100;
    let minute_hertz = hertz * 60;
    let hour_hertz = minute_hertz * 60;

    let mut table = Table::init(
        pids.iter().map(|p| {
            // When we have a tree, the name is prepended with an indent.
            let mut name = match tree {
                false => String::new(),
                true => name_indent.remove(&p.stat.pid).unwrap()
            };

            // For long output, try using the cmdline first.
            // FIXME: Sometimes prog_name != cmdline[0].
            if !long {
                name.push_str(&p.stat.comm);
            } else {
                let cmdline = p.cmdline.join(" ");
                name.push_str(
                    match cmdline {
                         ref s if s.len() > 0 => s,
                        _ => &p.stat.comm
                    }
                );
            }

            match (long, perf) {
                (_, false) => {
                    row![p.stat.pid, p.stat.ppid, name]
                },
                (_, true) => {
                    let rss = p.status.vmrss.map(|m| (m / 1024).to_string()).unwrap_or("".to_owned());
                    let raw_time = p.stat.utime + p.stat.stime;
                    let second_utime = raw_time / hertz % 60;
                    let minute_utime = raw_time / minute_hertz % 60;
                    let hour_utime = raw_time / hour_hertz % 60;
                    let cputime = format!(
                        "{:02}:{:02}:{:02}",
                        hour_utime,
                        minute_utime,
                        second_utime
                    );
                    row![p.stat.pid, p.stat.ppid, rss, cputime, name]
                }
            }
        }).collect::<Vec<_>>()
    );

    table.set_titles(
        match (long, perf) {
            (false, false) =>
                row!["Pid", "Ppid", "Cmd"],
            (true, false) =>
                row!["Pid", "Ppid", "Cmd"],
            (false, true) =>
                row!["Pid", "Ppid", "RSS", "Time", "Cmd"],
            (true, true) =>
                row!["Pid", "Ppid", "RSS", "Time", "Cmd"]
        }
    );
    table.set_format(
        FormatBuilder::new()
            .column_separator(' ')
            .build()
    );
    table.printstd();
}

// Given a vector of Pid structs, treeify their names, and return them in the right order.
// This is similar to ps -AH.
fn treeify_names(pids: Vec<Pid>, name_indents: &mut HashMap<TaskId, String>) -> Vec<Pid> {
    let mut child_pids = HashMap::new();
    for pid in pids {
        let ppid = pid.stat.ppid;
        child_pids.entry(ppid)
            .or_insert(Vec::new())
            .push(pid);
    }
    enumerate_children(0, &mut child_pids, name_indents, -1)
}

// Enumerate children pids, and return them.
fn enumerate_children(pid: TaskId, child_pids: &mut HashMap<TaskId, Vec<Pid>>,
    name_indents: &mut HashMap<TaskId, String>, indent: i32) -> Vec<Pid> {
    name_indents.insert(pid,
        match indent {
            i if i >= 0 =>
                repeat("  ").take(i as usize).collect::<String>(),
            _ => "".to_owned()
        }
    );
    let mut pids = Vec::new();
    let ppids = match child_pids.remove(&pid) {
        Some(v) => v,
        None => { return pids; }
    };
    for pid in ppids {
        let pid_num = pid.stat.pid;
        pids.push(pid);
        pids.append(
            &mut enumerate_children(pid_num, child_pids, name_indents, indent + 1)
        );
    }
    pids
}

struct ProgOpts {
    query: PidQuery,
    tree: bool,
    perf: bool,
    long: bool,
    verbose: bool
}

fn parse_args() -> ProgOpts {
    let mut opts = ProgOpts {
        query: PidQuery::NoneQuery,
        tree: false,
        perf: false,
        long: false,
        verbose: false
    };

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Query linux processes");
        ap.refer(&mut opts.tree)
            .add_option(&["-t", "--tree"], StoreTrue, "Display process tree");
        ap.refer(&mut opts.perf)
            .add_option(&["-p", "--perf"], StoreTrue, "Display performance information");
        ap.refer(&mut opts.long)
            .add_option(&["-l", "--long"], StoreTrue, "Display more information");
        ap.refer(&mut opts.verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
        ap.refer(&mut opts.query)
            .add_argument("query", Store, "Optional query to search by, pid or string");
        ap.parse_args_or_exit();
    }

    opts
}
