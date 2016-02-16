use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::collections::HashMap;
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

macro_rules! extract_key {
  ($map:expr, $key:expr, $func:expr) =>
    (try!(
      $map.remove($key)
        .and_then(|o|
          $func(o)
        )
        .ok_or(ProcHardError(ProcParseError, ProcPartStatus))
    ))
}

impl ProcStatus {
  // Generate ProcStatus struct given a process directory
  pub fn new(proc_dir: &str) -> Result<Self, ProcError> {
    // Try opening file
    let status_file = try!(
      File::open(Path::new(proc_dir).join("status"))
        .or(Err(ProcSoftError(ProcReadError, ProcPartStatus)))
    );

    let mut status: HashMap<String, String> =
      BufReader::new(status_file).lines().filter_map(
        |line|
          line
            .or(Err(ProcSoftError(ProcReadError, ProcPartStatus)))
            .and_then(|line| {
              let split = line.splitn(2, ':').collect::<Vec<&str>>();

              match (split.get(0), split.get(1)) {
                (Some(key), Some(value)) =>
                  Ok((key.trim().to_owned(),
                     value.trim().to_owned())),
                _ => Err(ProcHardError(ProcParseError, ProcPartStatus))
              }
             }).ok()
      ).collect();

    Ok(ProcStatus{
      pid: extract_key!(status, "Pid", parse_taskid),
      ppid: extract_key!(status, "PPid", parse_taskid),
      tgid: extract_key!(status, "Tgid", parse_taskid),
      name: extract_key!(status, "Name", |a| Some(a)),
      uid : extract_key!(status, "Uid", parse_uids),
      gid : extract_key!(status, "Gid", parse_uids)
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
