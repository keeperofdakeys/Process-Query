use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fmt;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::collections::HashMap;
use std::cmp::Ordering;

pub type TaskId = i32;

fn err_str<T: ToString>(err: T) -> String {
  err.to_string()
}

fn parse_taskid(taskid_str: String) -> Result<TaskId, String> {
  taskid_str.parse().map_err(err_str)
}

fn parse_uids(uid_str: String) -> Result<(u32, u32, u32, u32), String> {
  let uids = try!(
    uid_str.split("\t")
      .filter(|s| s != &"")
      .map(|s|
        s.parse()
      ).collect::<Result<Vec<_>, _>>()
      .map_err(err_str)
  );
  if uids.len() != 4 {
    return Err("Error parsing UIDs".to_owned());
  }
  Ok((uids[0], uids[1], uids[2], uids[3]))
}

#[derive(Debug)]
pub struct Proc {
  pub stat: Box<ProcStat>,
  pub status: Box<ProcStatus>,
  pub cmdline: Vec<String>
}

use ProcPart::*;

// Fields in a Proc
#[derive(Clone)]
enum ProcPart {
  ProcPartStat,
  ProcPartStatus,
  ProcPartCmdline
}

impl fmt::Display for ProcPart {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}",
      match *self {
        ProcPartStat => "stat",
        ProcPartStatus => "status",
        ProcPartCmdline => "cmdline"
      }
    )
  }
}

use ProcErrorType::*;

// Error types that can occur making a Proc
#[derive(Clone)]
enum ProcErrorType {
  ProcParseError,
  ProcReadError,
}

impl fmt::Display for ProcErrorType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}",
      match *self {
        ProcParseError => "parsing the file",
        ProcReadError => "reading the file"
      }
    )
  }
}

use ProcError::*;

// An error that occurs during parsing
#[derive(Clone)]
enum ProcError {
  // A soft error is something that is temporary, or recoverable.
  // For example, trying to read a /proc file for an invalid pid.
  ProcSoftError(ProcErrorType, ProcPart, String),
  // A hard error is something that is unrecoverable.
  // For example, a missing /proc, or a parsing error.
  ProcHardError(ProcErrorType, ProcPart, String),
}

impl fmt::Display for ProcError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let (e_type, part, error);
    let diag = match *self {
      ProcError::ProcHardError(ref t, ref p, ref s) => {
        e_type = t;
        part = p;
        error = s;
        "hard"
      },
      ProcError::ProcSoftError(ref t, ref p, ref s) => {
        e_type = t;
        part = p;
        error = s;
        "soft"
      }
    };
    write!(f, "A '{}' error occured while '{}' the '{}' part: {}",
      diag, e_type, part, error)
  }
}

impl Proc {
  pub fn new(pid: TaskId) -> Result<Self, String> {
    let proc_dir = format!("/proc/{}", pid);
    let proc_stat = try!(ProcStat::new(&proc_dir).map_err(err_str));
    // Once we have stat, we aren't too mindful if we miss the rest.
    let proc_status = try!(ProcStatus::new(&proc_dir));
    let cmdline = try!(Self::read_cmdline(&proc_dir).map_err(err_str));

    let proc_struct = Proc{
      stat: Box::new(proc_stat),
      status: Box::new(proc_status),
      cmdline: cmdline
    };

    Ok(proc_struct)
  }

  fn read_cmdline(proc_dir: &str) -> Result<Vec<String>, ProcError> {
    File::open(Path::new(proc_dir).join("cmdline"))
      .map_err(|e| ProcSoftError(ProcReadError, ProcPartCmdline, e.to_string()))
      .and_then(|mut file| {
        let mut contents = Vec::new();
        try!(
          file.read_to_end(&mut contents)
            .map_err(|e|
              ProcSoftError(ProcReadError,
                ProcPartCmdline, e.to_string()))
        );
        if contents.ends_with(&['\0' as u8]) {
          let _ = contents.pop();
        }
        Ok(contents)
      }).and_then(|contents| {
        String::from_utf8(contents)
          .map_err(|e|
            ProcSoftError(ProcParseError,
              ProcPartCmdline, e.to_string()))
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
      ProcQuery::CmdlineQuery(ref q) => string_query(&self.cmdline.join(" "), &q),
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

macro_rules! stat_parse_num {
  ($item:expr, $err: expr) =>
    (try!(
      $item.ok_or($err)
      .and_then(|s|
         s.parse()
           .map_err(|e: std::num::ParseIntError|
             ProcHardError(ProcParseError,
               ProcPartStat, e.to_string())
           )
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
  fn new(proc_dir: &str) -> Result<Self, ProcError> {
    let file = try!(
      File::open(Path::new(proc_dir).join("stat"))
        .map_err(|e|
          ProcSoftError(ProcReadError, ProcPartStat, e.to_string()))
    );
    let bytes = try!(
      file.bytes().collect::<Result<Vec<_>, _>>()
        .map_err(|e|
          ProcSoftError(ProcReadError, ProcPartStat, e.to_string()))
        .and_then(|s|
          String::from_utf8(s)
          .map_err(|e|
            ProcSoftError(ProcParseError, ProcPartStat, e.to_string()))
        )
    );
    // /proc/.../stat is "numbers (prog_name) char numbers"
    // prog_name could have arbitrary characters, so we need to parse
    // the file from both ends
    let read_error = ProcHardError(
      ProcParseError, ProcPartStat,
      "Error splitting file".to_owned()
    );
    let mut bytes_split = bytes.splitn(2, '(');
    let prefix = try!(bytes_split.next().ok_or(read_error.clone()));
    let mut bytes_split = bytes_split.next().unwrap().rsplitn(2, ')');
    // /proc/.../stat has a newline at the end
    let suffix = try!(bytes_split.next().ok_or(read_error.clone())).trim();
    let prog_name = try!(bytes_split.next().ok_or(read_error.clone()));

    let mut split = suffix.split(' ');

    Ok(ProcStat {
      pid: stat_parse_num!(prefix.split(' ').next(), read_error.clone()),
      // From here parse from back, since arbitrary data can be in program name
      comm: prog_name.to_owned(),
      state: try!(
        split.next()
          .and_then(|s|
            get_procstate(s)
          ).ok_or(read_error.clone())
      ),
      ppid: stat_parse_num!(split.next(), read_error.clone()),
      pgrp: stat_parse_num!(split.next(), read_error.clone()),
      session: stat_parse_num!(split.next(), read_error.clone()),
      tty_nr: stat_parse_num!(split.next(), read_error.clone()),
      tpgid: stat_parse_num!(split.next(), read_error.clone()),
      flags: stat_parse_num!(split.next(), read_error.clone()),
      minflt: stat_parse_num!(split.next(), read_error.clone()),
      cminflt: stat_parse_num!(split.next(), read_error.clone()),
      majflt: stat_parse_num!(split.next(), read_error.clone()),
      cmajflt: stat_parse_num!(split.next(), read_error.clone()),
      utime: stat_parse_num!(split.next(), read_error.clone()),
      stime: stat_parse_num!(split.next(), read_error.clone()),
      cutime: stat_parse_num!(split.next(), read_error.clone()),
      cstime: stat_parse_num!(split.next(), read_error.clone()),
      priority: stat_parse_num!(split.next(), read_error.clone()),
      nice: stat_parse_num!(split.next(), read_error.clone()),
      num_threads: stat_parse_num!(split.next(), read_error.clone()),
      itrealvalue: stat_parse_num!(split.next(), read_error.clone()),
      starttime: stat_parse_num!(split.next(), read_error.clone()),
      vsize: stat_parse_num!(split.next(), read_error.clone()),
      rss: stat_parse_num!(split.next(), read_error.clone()),
      rsslim: stat_parse_num!(split.next(), read_error.clone()),
      startcode: stat_parse_num!(split.next(), read_error.clone()),
      endcode: stat_parse_num!(split.next(), read_error.clone()),
      startstack: stat_parse_num!(split.next(), read_error.clone()),
      kstkesp: stat_parse_num!(split.next(), read_error.clone()),
      kstkeip: stat_parse_num!(split.next(), read_error.clone()),
      signal: stat_parse_num!(split.next(), read_error.clone()),
      blocked: stat_parse_num!(split.next(), read_error.clone()),
      sigignore: stat_parse_num!(split.next(), read_error.clone()),
      sigcatch: stat_parse_num!(split.next(), read_error.clone()),
      wchan: stat_parse_num!(split.next(), read_error.clone()),
      nswap: stat_parse_num!(split.next(), read_error.clone()),
      cnswap: stat_parse_num!(split.next(), read_error.clone()),
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
  // uid: Real, Effective, Saved, Filesystem
  pub uid: (u32, u32, u32, u32),
  // gid: Real, Effective, Saved, Filesystem
  pub gid: (u32, u32, u32, u32)
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

    Ok(ProcStatus{
      pid: extract_key!(status, "Pid", parse_taskid),
      ppid: extract_key!(status, "PPid", parse_taskid),
      tgid: extract_key!(status, "Tgid", parse_taskid),
      name: extract_key!(status, "Name", |a| Ok(a)),
      uid : extract_key!(status, "Uid", parse_uids),
      gid : extract_key!(status, "Gid", parse_uids)
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
    -> Option<Result<Proc, String>> {
    // TODO: This sucks, find a better way
    let file = entry_opt
      .map_err(err_str)
      .and_then(|entry|
        entry.file_name().into_string()
          .or(Err("Error parsing filename".to_owned()))
      );

    if file.is_err() {
      return None;
    }

    match file.unwrap().parse() {
      Ok(pid) => {
        let proc_s_r = Proc::new(pid);
        if proc_s_r.is_err() {
          return Some(proc_s_r);
        }
        let proc_s = proc_s_r.unwrap();
        match proc_s.query(query) {
          true => Some(Ok(proc_s)),
          false => None
        }
      },
      Err(_) => None
    }
  }
}

impl Iterator for ProcIter {
  type Item = Result<Proc, String>;

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
  iter.map(|proc_s|
    proc_s.map(|p|
      (p.stat.pid, p)
    )
  ).collect()
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
  text.contains(query)
}
