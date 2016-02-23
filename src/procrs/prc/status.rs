use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::collections::HashMap;
use super::error::{PrcError, PrcFile};
use super::{TaskId, MemSize};

#[derive(Debug)]
pub struct PrcStatus {
  pub name: String,
  pub tgid: TaskId,
  pub pid: TaskId,
  pub ppid: TaskId,
  pub tracerpid: TaskId,
  // uid: Real, Effective, Saved, Filesystem
  pub uid: (u32, u32, u32, u32),
  // gid: Real, Effective, Saved, Filesystem
  pub gid: (u32, u32, u32, u32),
  pub fdsize: u32,
  pub vmpeak: Option<MemSize>,
  pub vmsize: Option<MemSize>,
  pub vmlck: Option<MemSize>,
  pub vmpin: Option<MemSize>,
  pub vmhwm: Option<MemSize>,
  pub vmrss: Option<MemSize>,
  pub vmdata: Option<MemSize>,
  pub vmstk: Option<MemSize>,
  pub vmexe: Option<MemSize>,
  pub vmlib: Option<MemSize>,
  pub vmpte: Option<MemSize>,
  pub vmpmd: Option<MemSize>,
  pub vmswap: Option<MemSize>,
  pub threads: u32
}

macro_rules! extract_line_opt {
  ($map:expr, $key:expr, $func:expr) =>
    ($map.remove($key)
      .and_then($func)
    )
}

macro_rules! extract_line {
  ($map:expr, $key:expr, $func:expr) =>
    (try!(
      extract_line_opt!($map, $key, $func)
        .ok_or(PrcError::Field(PrcFile::PrcStat, $key))
    ))
}

impl PrcStatus {
  // Generate PrcStatus struct given a process directory
  pub fn new(proc_dir: &str) -> Result<Self, PrcError> {
    // Try opening file
    let status_file = try!(
      File::open(Path::new(proc_dir).join("status"))
        .map_err(|e| PrcError::Opening(PrcFile::PrcStatus, e))
    );

    let lines =
      BufReader::new(status_file)
        .lines()
        .map(|r|
          match r {
            Ok(o) => Ok(o),
            Err(e) => Err(PrcError::Reading(PrcFile::PrcStatus, e))
          }
        );
    Self::parse_string(lines)
  }

  fn parse_string<I: Iterator<Item=Result<String, PrcError>>>(lines: I) -> Result<Self, PrcError> {
    let mut status: HashMap<_, _> = 
      try!(
        lines.map(|r|
          match r {
            Ok(line) => {
              let split = line.splitn(2, ':').collect::<Vec<&str>>();
              match (split.get(0), split.get(1)) {
                (Some(key), Some(value)) =>
                  Ok((key.trim().to_owned(),
                     value.trim().to_owned())),
                _ => Err(PrcError::Parsing(PrcFile::PrcStatus, "No colon on line"))
              }
            },
            Err(e) => Err(e)
          }
        ).collect::<Result<_, _>>());


    // It's quite important that these appear in the order that they
    // appear in the status file
    Ok(PrcStatus{
      name: extract_line!(status, "Name", |s| Some(s)),
      tgid: extract_line!(status, "Tgid", |s| s.parse().ok()),
      pid: extract_line!(status, "Pid", |s| s.parse().ok()),
      ppid: extract_line!(status, "PPid", |s| s.parse().ok()),
      tracerpid: extract_line!(status, "TracerPid", |s| s.parse().ok()),
      uid : extract_line!(status, "Uid", parse_uids),
      gid : extract_line!(status, "Gid", parse_uids),
      fdsize : extract_line!(status, "FDSize", |s| s.parse().ok()),
      vmpeak: extract_line_opt!(status, "VmPeak", parse_mem),
      vmsize: extract_line_opt!(status, "VmSize", parse_mem),
      vmlck: extract_line_opt!(status, "VmLck", parse_mem),
      vmpin: extract_line_opt!(status, "VmPin", parse_mem),
      vmhwm: extract_line_opt!(status, "VmHWM", parse_mem),
      vmrss: extract_line_opt!(status, "VmRSS", parse_mem),
      vmdata: extract_line_opt!(status, "VmData", parse_mem),
      vmstk: extract_line_opt!(status, "VmStk", parse_mem),
      vmexe: extract_line_opt!(status, "VmExe", parse_mem),
      vmlib: extract_line_opt!(status, "VmLib", parse_mem),
      vmpmd: extract_line_opt!(status, "VmPMD", parse_mem),
      vmpte: extract_line_opt!(status, "VmPTE", parse_mem),
      vmswap: extract_line_opt!(status, "VmSwap", parse_mem),
      threads: extract_line!(status, "Threads", |s| s.parse().ok())
    })
  }
}

fn parse_uids(uid_str: String) -> Option<(u32, u32, u32, u32)> {
  let uids = match
    uid_str.split("\t")
      .filter(|s| s != &"")
      .map(|s|
        s.parse()
      ).collect::<Result<Vec<_>, _>>()
      .ok()
    {
      Some(s) => s,
      None => return None
    };
  if uids.len() != 4 {
    return None;
  }
  Some((uids[0], uids[1], uids[2], uids[3]))
}

fn parse_mem(mem_str: String) -> Option<MemSize> {
  Some(1)
}
