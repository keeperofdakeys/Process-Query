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
  pub status: ProcStatus,
  pub cmdline: Vec<String>
}

impl Proc {
  pub fn new(pid: TaskId) -> Result<Self, String> {
    let proc_dir = format!("/proc/{}", pid);
    let proc_status = try!(ProcStatus::new(&proc_dir));
    let cmdline = try!(Self::read_cmdline(&proc_dir));

    let proc_struct = Proc{
      status: proc_status,
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
        contents.split('\0')
          .map(|a| a.to_string())
          .collect()
      )
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
                  Ok((key.trim().to_string(),
                     value.trim().to_string())),
                _ => Err("Error reading status line".to_string())
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
    self.status.pid.eq(&other.status.pid)
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
    self.status.pid.cmp(&other.status.pid)
  }
}

struct ProcIter {
  dir_iter: ReadDir
}

impl ProcIter {
  pub fn new() -> Result<Self, String> {
    let proc_dir = Path::new("/proc");
    let dir_iter = try!(fs::read_dir(proc_dir).map_err(err_str));
    Ok(ProcIter{
      dir_iter: dir_iter
    })
  }

  fn proc_dir_filter(entry_opt: Result<DirEntry, io::Error>) -> Option<Proc> {
    entry_opt.ok()
      .and_then(|entry|
        entry.file_name().into_string().ok()
      ).and_then(|name|
        name.parse().ok()
      ).and_then(|pid|
        Proc::new(pid).ok()
      )
  }
}

impl Iterator for ProcIter {
  type Item = Proc;

  fn next(&mut self) -> Option<Self::Item> {
    for entry in self.dir_iter.by_ref() {
      match (Self::proc_dir_filter)(entry) {
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
    (proc_struct.status.pid, proc_struct)
  ).collect())
}
