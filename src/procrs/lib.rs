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
        contents
          .split('\0')
          .map(|a| a.to_string())
          .collect()
      )
  }

  // Return true if query matches this process
  fn query(&self, query: &ProcQuery) -> bool {
    match *query {
      ProcQuery::PidQuery(q) => taskid_query(self.status.pid, q),
      ProcQuery::PpidQuery(q) => taskid_query(self.status.ppid, q),
      ProcQuery::NameQuery(ref q) => string_query(&self.status.name, &q),
      ProcQuery::CmdlineQuery(ref q) => string_s_query(&self.cmdline, &q),
      ProcQuery::NoneQuery => true
    }
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
    (proc_struct.status.pid, proc_struct)
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


