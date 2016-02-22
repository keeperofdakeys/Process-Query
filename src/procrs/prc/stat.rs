use std::fs::File;
use std::path::Path;
use std::io::{Read, BufReader};
use super::error::{PrcError, PrcFile};
use super::TaskId;

#[derive(Debug, Clone, PartialEq)]
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
        PrcError::Parsing(PrcFile::PrcStat, "missing field")
      ).and_then(|s|
         s.parse()
           .or(Err(PrcError::Parsing(PrcFile::PrcStat, "parsing number")))
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
  pub fn new(proc_dir: &str) -> Result<Self, PrcError> {
    let file = try!(
      File::open(Path::new(proc_dir).join("stat"))
        .map_err(|e| PrcError::Opening(PrcFile::PrcStat, e))
    );
    let bytes = try!(BufReader::new(file)
      .bytes().collect::<Result<Vec<_>, _>>()
      .map_err(|e| PrcError::Reading(PrcFile::PrcStat, e))
      .and_then(|s|
        String::from_utf8(s)
        .or(Err(PrcError::Parsing(PrcFile::PrcStat, "converting to utf8")))
      )
    );
    Self::parse_string(bytes)
  }

  fn parse_string(bytes: String) -> Result<Self, PrcError> {
    // /proc/.../stat is "numbers (prog_name) char numbers"
    // prog_name could have arbitrary characters, so we need to parse
    // the file from both ends
    let mut bytes_split = bytes.splitn(2, '(');
    let prefix = try!(bytes_split.next()
      .ok_or(PrcError::Parsing(PrcFile::PrcStat, "finding opening paren")));
    let mut bytes_split = match bytes_split.next() {
      Some(b) => b.rsplitn(2, ')'),
      None => return Err(PrcError::Parsing(PrcFile::PrcStat, "finding closing paren"))
    };
    // /proc/.../stat has a newline at the end
    let suffix = try!(bytes_split.next()
      .ok_or(PrcError::Parsing(PrcFile::PrcStat, "splitting file"))).trim();
    let prog_name = try!(bytes_split.next()
      .ok_or(PrcError::Parsing(PrcFile::PrcStat, "splitting comm")));
    let mut split = suffix.split(' ');

    Ok(ProcStat {
      pid: stat_parse_num!(prefix.split(' ').next()),
      // From here parse from back, since arbitrary data can be in program name
      comm: prog_name.to_owned(),
      state: try!(
        split.next()
          .and_then(|s|
            get_procstate(s)
          ).ok_or(PrcError::Parsing(PrcFile::PrcStat, "parsing process state"))
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

#[derive(Debug, Clone, PartialEq)]
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

#[test]
fn test_parsing() {
  let test_prc = ProcStat{
    pid: 14557,
    comm: "psq".to_owned(),
    state: ProcState::Stopped,
    ppid: 14364,
    pgrp: 14557,
    session: 14364,
    tty_nr: 34823,
    tpgid: 14638,
    flags: 1077952512,
    minflt: 1178,
    cminflt: 0,
    majflt: 0,
    cmajflt: 0,
    utime: 16,
    stime: 0,
    cutime: 0,
    cstime: 0,
    priority: 20,
    nice: 0,
    num_threads: 1,
    itrealvalue: 0,
    starttime: 609164,
    vsize: 23785472,
    rss: 1707,
    rsslim: 18446744073709551615,
    startcode: 94178658361344,
    endcode: 94178659818816,
    startstack: 140735096462144,
    kstkesp: 140735096450384,
    kstkeip: 94178659203252,
    signal: 0,
    blocked: 0,
    sigignore: 4224,
    sigcatch: 1088,
    wchan: 1,
    nswap: 0,
    cnswap: 0,
    exit_signal: Some(17),
    processor: Some(2),
    rt_priority: Some(0),
    policy: Some(0),
    delayacct_blkio_ticks: Some(0),
    guest_time: Some(0),
    cguest_time: Some(0),
    start_data: Some(94178661916280),
    end_data: Some(94178661971297),
    start_brk: Some(94178690334720),
    arg_start: Some(140735096465030),
    arg_end: Some(140735096465049),
    env_start: Some(140735096465049),
    env_end: Some(140735096467429),
    exit_code: Some(0)
  };

  let input = "14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Ok(test_prc));
}

// For each of the following tests, the previous text input is used to create a ProcStat struct.

#[test]
fn test_state_running() {
  let mut prc = ProcStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
  prc.state = ProcState::Running;
  let input = "14557 (psq) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Ok(prc));
}

#[test]
fn test_comm_space() {
  let mut prc = ProcStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
  prc.state = ProcState::Running;
  prc.comm = "psq ".to_owned();
  let input = "14557 (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Ok(prc));
}

#[test]
fn test_double_space() {
  let mut prc = ProcStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
  prc.state = ProcState::Running;
  prc.comm = "psq ".to_owned();
  let input = "14557  (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Ok(prc));
}

#[test]
fn test_comm_parens() {
  let mut prc = ProcStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
  prc.state = ProcState::Running;
  prc.comm = " ) (psq ".to_owned();
  let input = "14557  ( ) (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Ok(prc));
}

#[test]
fn test_invalid_parens() {
  let input = "14557   ) (psq (R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
  assert_eq!(ProcStat::parse_string(input), Err(PrcError::Parsing(PrcFile::PrcStat, "splitting comm")));
}

#[test]
fn test_invalid_1() {
  let input = "14557 ".to_owned();
  assert_eq!(ProcStat::parse_string(input), Err(PrcError::Parsing(PrcFile::PrcStat, "finding closing paren")));
}

#[test]
fn test_invalid_2() {
  let input = "14557 (a) 3".to_owned();
  assert_eq!(ProcStat::parse_string(input), Err(PrcError::Parsing(PrcFile::PrcStat, "parsing process state")));
}
