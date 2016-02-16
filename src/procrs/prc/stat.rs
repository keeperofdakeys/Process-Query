use std::fs::File;
use std::path::Path;
use std::io::Read;
use super::error::*;
use super::TaskId;

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
  ($item:expr) =>
    (try!(
      $item.ok_or(
        ProcHardError(ProcParseError, ProcPartStat)
      ).and_then(|s|
         s.parse()
           .or(Err(ProcHardError(ProcParseError, ProcPartStat)))
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
  pub fn new(proc_dir: &str) -> Result<Self, ProcError> {
    let file = try!(
      File::open(Path::new(proc_dir).join("stat"))
        .or(Err(ProcSoftError(ProcReadError, ProcPartStat)))
    );
    let bytes = try!(
      file.bytes().collect::<Result<Vec<_>, _>>()
        .or(Err(ProcSoftError(ProcReadError, ProcPartStat)))
        .and_then(|s|
          String::from_utf8(s)
          .or(Err(ProcSoftError(ProcParseError, ProcPartStat)))
        )
    );
    // /proc/.../stat is "numbers (prog_name) char numbers"
    // prog_name could have arbitrary characters, so we need to parse
    // the file from both ends
    let read_error = ProcHardError(ProcParseError, ProcPartStat);
    let mut bytes_split = bytes.splitn(2, '(');
    let prefix = try!(bytes_split.next().ok_or(read_error.clone()));
    let mut bytes_split = bytes_split.next().unwrap().rsplitn(2, ')');
    // /proc/.../stat has a newline at the end
    let suffix = try!(bytes_split.next().ok_or(read_error.clone())).trim();
    let prog_name = try!(bytes_split.next().ok_or(read_error.clone()));

    let mut split = suffix.split(' ');

    Ok(ProcStat {
      pid: stat_parse_num!(prefix.split(' ').next()),
      // From here parse from back, since arbitrary data can be in program name
      comm: prog_name.to_owned(),
      state: try!(
        split.next()
          .and_then(|s|
            get_procstate(s)
          ).ok_or(ProcHardError(ProcParseError, ProcPartStat))
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
pub enum ProcState {
  Running,
  Sleeping,
  Waiting,
  Zombie,
  Stopped,
  Tracing,
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
