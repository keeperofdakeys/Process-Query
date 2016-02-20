use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use super::error::*;
use super::TaskId;

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

macro_rules! extract_line {
  ($iter:expr, $key:expr, $func:expr) =>
    (try!(
      match $iter.find(|r| {
          match *r {
            Ok((ref k, _)) if k == $key => true,
            Err(_) => true,
            _ => false
          }
      }) {
        Some(Ok((_, v))) => Ok($func(v)),
        Some(Err(e)) => Err(e),
        None => Err(ProcHardError(ProcParseError, ProcPartStatus))
      }
    ).unwrap())
}

impl ProcStatus {
  // Generate ProcStatus struct given a process directory
  pub fn new(proc_dir: &str) -> Result<Self, ProcError> {
    // Try opening file
    let status_file = try!(
      File::open(Path::new(proc_dir).join("status"))
        .or(Err(ProcSoftError(ProcReadError, ProcPartStatus)))
    );

    let lines =
      BufReader::new(status_file)
        .lines()
        .map(|r|
          match r {
            Ok(o) => Ok(o),
            Err(_) => Err(ProcSoftError(ProcReadError, ProcPartStatus))
          }
        );
    Self::parse_string(lines)
  }

  fn parse_string<I: Iterator<Item=Result<String, ProcError>>>(lines: I) -> Result<Self, ProcError> {
    let mut status = lines
      .map(|r|
        match r {
          Ok(line) => {
            let split = line.splitn(2, ':').collect::<Vec<&str>>();
            match (split.get(0), split.get(1)) {
              (Some(key), Some(value)) =>
                Ok((key.trim().to_owned(),
                   value.trim().to_owned())),
              _ => Err(ProcHardError(ProcParseError, ProcPartStatus))
            }
          },
          Err(_) => Err(ProcSoftError(ProcReadError, ProcPartStatus))
        }
      );

    // It's quite important that these appear in the order that they
    // appear in the status file
    Ok(ProcStatus{
      name: extract_line!(status, "Name", |a| Some(a)),
      tgid: extract_line!(status, "Tgid", parse_taskid),
      pid: extract_line!(status, "Pid", parse_taskid),
      ppid: extract_line!(status, "PPid", parse_taskid),
      uid : extract_line!(status, "Uid", parse_uids),
      gid : extract_line!(status, "Gid", parse_uids)
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

fn parse_taskid(taskid_str: String) -> Option<TaskId> {
  taskid_str.parse().ok()
}
