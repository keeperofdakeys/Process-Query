use std::io::prelude::*;
use std::io::BufReader;
use std::fs::{self, DirEntry, File};
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

pub type ProcMap = HashMap<TaskId, Proc>;

pub fn get_proc_map() -> Result<ProcMap, String> {
  let proc_dir = Path::new("/proc");

  let mut proc_map = HashMap::new();
  for entry in try!(fs::read_dir(proc_dir).map_err(err_str)) {
    let name = try!(
      entry
        .map(|name|
          name.file_name()
        ).map_err(err_str)
        .and_then(|name|
          name.into_string()
            .or(Err("Invalid dir name".to_string()))
        )
    );
    let pid = match name.parse() {
      Ok(pid) => pid,
      Err(_) => continue
    };
    let proc_struct = try!(Proc::new(pid));
    proc_map.insert(pid, proc_struct);
  }
  Ok(proc_map)
}
