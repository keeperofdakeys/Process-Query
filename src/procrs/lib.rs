use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::collections::HashMap;
use std::cmp::Ordering;

pub type TaskId = u32;

fn err_str<T: ToString>(err: T) -> String {
  err.to_string()
}

fn parse_taskid(taskid_str: String) -> Result<TaskId, String> {
  taskid_str.parse().map_err(err_str)
}

#[derive(Debug)]
pub struct Proc {
  pub stat: Box<ProcStat>,
  pub status: Box<ProcStatus>,
  pub cmdline: Vec<String>
}

impl Proc {
  pub fn new(pid: TaskId) -> Result<Self, String> {
    let proc_dir = format!("/proc/{}", pid);
    let proc_stat = try!(ProcStat::new(&proc_dir));
    let proc_status = try!(ProcStatus::new(&proc_dir));
    let cmdline = try!(Self::read_cmdline(&proc_dir));

    let proc_struct = Proc{
      stat: Box::new(proc_stat),
      status: Box::new(proc_status),
      cmdline: cmdline
    };

    Ok(proc_struct)
  }

  fn read_cmdline(proc_dir: &str) -> Result<Vec<String>, String> {
    File::open(Path::new(proc_dir).join("cmdline"))
      .map_err(err_str)
      .and_then(|mut file| {
        let mut contents = Vec::new();
        try!(
          file.read_to_end(&mut contents)
            .map_err(err_str)
        );
        if contents.ends_with(&['\0' as u8]) {
          let _ = contents.pop();
        }
        Ok(contents)
      }).and_then(|contents| {
        String::from_utf8(contents)
          .map_err(err_str)
      }).map(|contents|
        contents
          .split('\0')
          .map(|a| a.to_string())
          .collect()
      )
  }

  // Return true if query matches this process
  fn query(&self, query: &ProcQuery) -> bool {
    match *query {
      ProcQuery::PidQuery(q) => taskid_query(self.stat.pid, q),
      ProcQuery::PpidQuery(q) => taskid_query(self.stat.ppid, q),
      ProcQuery::NameQuery(ref q) => string_query(&self.stat.comm, &q),
      ProcQuery::CmdlineQuery(ref q) => string_s_query(&self.cmdline, &q),
      ProcQuery::NoneQuery => true
    }
  }
}

#[derive(Debug)]
pub enum ProcState {
  Running,
  Sleeping,
  Waiting,
  Zombie,
  Stopped,
  Tracing,
  Paging,
  Dead,
  Wakekill,
  Waking,
  Parked
}

fn get_procstate(state: &str) -> Option<ProcState> {
  match state {
    "R" => Some(ProcState::Running),
    "S" => Some(ProcState::Sleeping),
    "D" => Some(ProcState::Waiting),
    "Z" => Some(ProcState::Zombie),
    "T" => Some(ProcState::Stopped),
    "t" => Some(ProcState::Tracing),
    "X" | "x" => Some(ProcState::Dead),
    "K" => Some(ProcState::Wakekill),
    "W" => Some(ProcState::Waking),
    "P" => Some(ProcState::Parked),
     _  => None
  }
}

#[derive(Debug)]
pub struct ProcStat {
  pub pid: TaskId,
  pub comm: String,
  pub state: ProcState,
  pub ppid: TaskId,
  pub pgrp: i32,
  pub session: i32,
  pub tty_nr: i32,
  pub tpgid: i32,
  pub flags: u32,
  pub minflt: u64,
  pub cminflt: u64,
  pub majflt: u64,
  pub cmajflt: u64,
  pub utime: u64,
  pub stime: u64,
  pub cutime: i64,
  pub cstime: i64,
  pub priority: i64,
  pub nice: i64,
  pub num_threads: i64,
  pub itrealvalue: i64,
  pub starttime: u64,
  pub vsize: u64,
  pub rss: i64,
  pub rsslim: u64,
  pub startcode: u64,
  pub endcode: u64,
  pub startstack: u64,
  pub kstkesp: u64,
  pub kstkeip: u64,
  pub signal: u64,
  pub blocked: u64,
  pub sigignore: u64,
  pub sigcatch: u64,
  pub wchan: u64,
  pub nswap: u64,
  pub cnswap: u64,
  // These fields depend on kernel version (linux 2.1 -> 3.5), so wrap in Option
  pub exit_signal: Option<i32>,
  pub processor: Option<i32>,
  pub rt_priority: Option<u32>,
  pub policy: Option<u32>,
  pub delayacct_blkio_ticks: Option<u64>,
  pub guest_time: Option<u64>,
  pub cguest_time: Option<i64>,
  pub start_data: Option<u64>,
  pub end_data: Option<u64>,
  pub start_brk: Option<u64>,
  pub arg_start: Option<u64>,
  pub arg_end: Option<u64>,
  pub env_start: Option<u64>,
  pub env_end: Option<u64>,
  pub exit_code: Option<i32>
}

const READ_ERROR: &'static str = "Error parsing /proc/../stat file";

macro_rules! stat_parse_num {
  ($item:expr) =>
    (try!(
      $item.ok_or(READ_ERROR.to_owned())
      .and_then(|s|
         s.parse()
         .map_err(err_str)
      )
    ))
}

macro_rules! stat_parse_opt_num {
  ($item:expr) =>
    ($item.and_then(|s|
       s.parse().ok()
     ))
}

impl ProcStat {
  // Generate ProcStat struct given a process directory
  fn new(proc_dir: &str) -> Result<Self, String> {
    let file = try!(
      File::open(Path::new(proc_dir).join("stat"))
        .map_err(err_str)
    );
    let bytes = try!(
      file.bytes().collect::<Result<Vec<_>, _>>()
        .map_err(err_str)
        .and_then(|s|
          String::from_utf8(s)
          .map_err(err_str)
        )
    );
    // /proc/.../stat is "numbers (prog_name) char numbers"
    // prog_name could have arbitrary characters, so we need to parse
    // the file from both ends
    let mut bytes_split = bytes.splitn(2, '(');
    let prefix = try!(bytes_split.next().ok_or(READ_ERROR.to_owned()));
    let mut bytes_split = bytes_split.next().unwrap().rsplitn(2, ')');
    // /proc/.../stat has a newline at the end
    let suffix = try!(bytes_split.next().ok_or(READ_ERROR.to_owned())).trim();
    let prog_name = try!(bytes_split.next().ok_or(READ_ERROR.to_owned()));

    let mut split = suffix.split(' ');

    Ok(ProcStat {
      pid: stat_parse_num!(prefix.split(' ').next()),
      // From here parse from back, since arbitrary data can be in program name
      comm: prog_name.to_owned(),
      state: try!(
        split.next()
          .and_then(|s|
            get_procstate(s)
          ).ok_or(READ_ERROR.to_owned())
      ),
      ppid: stat_parse_num!(split.next()),
      pgrp: stat_parse_num!(split.next()),
      session: stat_parse_num!(split.next()),
      tty_nr: stat_parse_num!(split.next()),
      tpgid: stat_parse_num!(split.next()),
      flags: stat_parse_num!(split.next()),
      minflt: stat_parse_num!(split.next()),
      cminflt: stat_parse_num!(split.next()),
      majflt: stat_parse_num!(split.next()),
      cmajflt: stat_parse_num!(split.next()),
      utime: stat_parse_num!(split.next()),
      stime: stat_parse_num!(split.next()),
      cutime: stat_parse_num!(split.next()),
      cstime: stat_parse_num!(split.next()),
      priority: stat_parse_num!(split.next()),
      nice: stat_parse_num!(split.next()),
      num_threads: stat_parse_num!(split.next()),
      itrealvalue: stat_parse_num!(split.next()),
      starttime: stat_parse_num!(split.next()),
      vsize: stat_parse_num!(split.next()),
      rss: stat_parse_num!(split.next()),
      rsslim: stat_parse_num!(split.next()),
      startcode: stat_parse_num!(split.next()),
      endcode: stat_parse_num!(split.next()),
      startstack: stat_parse_num!(split.next()),
      kstkesp: stat_parse_num!(split.next()),
      kstkeip: stat_parse_num!(split.next()),
      signal: stat_parse_num!(split.next()),
      blocked: stat_parse_num!(split.next()),
      sigignore: stat_parse_num!(split.next()),
      sigcatch: stat_parse_num!(split.next()),
      wchan: stat_parse_num!(split.next()),
      nswap: stat_parse_num!(split.next()),
      cnswap: stat_parse_num!(split.next()),
      exit_signal:
        stat_parse_opt_num!(split.next()),
      processor:
        stat_parse_opt_num!(split.next()),
      rt_priority:
        stat_parse_opt_num!(split.next()),
      policy:
        stat_parse_opt_num!(split.next()),
      delayacct_blkio_ticks:
        stat_parse_opt_num!(split.next()),
      guest_time:
        stat_parse_opt_num!(split.next()),
      cguest_time:
        stat_parse_opt_num!(split.next()),
      start_data:
        stat_parse_opt_num!(split.next()),
      end_data:
        stat_parse_opt_num!(split.next()),
      start_brk:
        stat_parse_opt_num!(split.next()),
      arg_start:
        stat_parse_opt_num!(split.next()),
      arg_end:
        stat_parse_opt_num!(split.next()),
      env_start:
        stat_parse_opt_num!(split.next()),
      env_end:
        stat_parse_opt_num!(split.next()),
      exit_code:
        stat_parse_opt_num!(split.next()),
    })
  }
}

#[derive(Debug)]
pub struct ProcStatus {
  pub pid: TaskId,
  pub ppid: TaskId,
  pub tgid: TaskId,
  pub name: String,
}

macro_rules! extract_key {
  ($map:expr, $key:expr, $func:expr) =>
    (try!(
      $map.remove($key)
        .ok_or(format!("Key '{}' not found", $key))
        .and_then($func)
    ))
}

impl ProcStatus {
  // Generate ProcStatus struct given a process directory
  fn new(proc_dir: &str) -> Result<Self, String> {
    // Try opening file
    let status_file = try!(
      File::open(Path::new(proc_dir).join("status"))
        .map_err(err_str)
    );

    let mut status: HashMap<String, String> =
      BufReader::new(status_file).lines().filter_map(
        |line|
          line
            .map_err(err_str)
            .and_then(|line| {
              let split = line.splitn(2, ':').collect::<Vec<&str>>();

              match (split.get(0), split.get(1)) {
                (Some(key), Some(value)) =>
                  Ok((key.trim().to_owned(),
                     value.trim().to_owned())),
                _ => Err("Error reading status line".to_owned())
              }
             }).ok()
      ).collect();

    let pid = extract_key!(status, "Pid", parse_taskid);
    let ppid = extract_key!(status, "PPid", parse_taskid);
    let tgid = extract_key!(status, "Tgid", parse_taskid);
    let name = extract_key!(status, "Name", |a| Ok(a));

    Ok(ProcStatus{
      pid: pid,
      ppid: ppid,
      tgid: tgid,
      name: name
    })
  }
}

impl PartialEq for Proc {
  fn eq(&self, other: &Self) -> bool {
    self.stat.pid.eq(&other.stat.pid)
  }
}

impl Eq for Proc {}
impl PartialOrd for Proc {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
impl Ord for Proc {
  fn cmp(&self, other: &Self) -> Ordering {
    self.stat.pid.cmp(&other.stat.pid)
  }
}

pub struct ProcIter {
  dir_iter: ReadDir,
  query: ProcQuery
}

impl ProcIter {
  pub fn new() -> Result<Self, String> {
    Self::new_query(ProcQuery::NoneQuery)
  }

  pub fn new_query(query: ProcQuery) -> Result<Self, String> {
    let proc_dir = Path::new("/proc");
    let dir_iter = try!(fs::read_dir(proc_dir).map_err(err_str));
    Ok(ProcIter{
      dir_iter: dir_iter,
      query: query
    })
  }

  fn proc_dir_filter(entry_opt: Result<DirEntry, io::Error>, query: &ProcQuery)
    -> Option<Proc> {
    // TODO: This sucks, find a better way
    entry_opt.ok()
      .and_then(|entry|
        entry.file_name().into_string().ok()
      ).and_then(|name|
        name.parse().ok()
      ).and_then(|pid|
        Proc::new(pid).ok()
      ).and_then(|proc_struct|
        if proc_struct.query(query) {
          Some(proc_struct)
        } else {
          None
        }
      )
  }
}

impl Iterator for ProcIter {
  type Item = Proc;

  fn next(&mut self) -> Option<Self::Item> {
    for entry in self.dir_iter.by_ref() {
      match Self::proc_dir_filter(entry, &self.query) {
        Some(p) => return Some(p),
        None => continue
      }
    }
    None
  }

  // Size may be anywhere from 0 to number of dirs
  fn size_hint(&self) -> (usize, Option<usize>) {
    (0, self.dir_iter.size_hint().1)
  }
}

pub type ProcMap = HashMap<TaskId, Proc>;

pub fn get_proc_map() -> Result<ProcMap, String> {
  let iter = try!(ProcIter::new());
  Ok(iter.map(|proc_struct|
    (proc_struct.stat.pid, proc_struct)
  ).collect())
}

pub enum ProcQuery {
  PidQuery(TaskId),
  PpidQuery(TaskId),
  NameQuery(String),
  CmdlineQuery(String),
  NoneQuery
}

pub fn create_query(query: &str) -> Result<ProcQuery, String> {
  let splits: Vec<_> = query.splitn(2, '=').collect();

  match splits.len() {
    0 => Ok(ProcQuery::NoneQuery),
    1 => Ok(match query.parse().ok() {
      Some(tid) => ProcQuery::PidQuery(tid),
      None => ProcQuery::NameQuery(query.to_owned())
    }),
    _ => {
      let q_text = splits[1].to_owned();
      let q_tid = q_text.parse();
      match &*splits[0].to_lowercase() {
        "pid" => q_tid.map(|q| ProcQuery::PidQuery(q))
          .or(Err("Query value for type 'pid' not valid".to_owned())),
        "ppid" => q_tid.map(|q| ProcQuery::PpidQuery(q))
          .or(Err("Query value for type 'ppid' not valid".to_owned())),
        "name" => Ok(ProcQuery::NameQuery(q_text)),
        "cmdline" => Ok(ProcQuery::CmdlineQuery(q_text)),
        _ => Err("Invalid query type".to_owned())
      }
    }
  }
}

pub fn taskid_query(tid: TaskId, query: TaskId) -> bool {
  tid == query
}

pub fn string_query(text: &str, query: &str) -> bool {
  text == query
}

pub fn string_s_query(text_vec: &Vec<String>, query: &str) -> bool {
  for text in text_vec {
    if string_query(text, query) {
      return true;
    }
  }
  false
}


