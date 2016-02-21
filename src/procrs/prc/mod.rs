use std::io;
use std::io::prelude::*;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::str::FromStr;

mod stat;
mod status;
pub mod error;

use self::stat::ProcStat;
use self::status::ProcStatus;
use self::error::*;

pub type TaskId = i32;
pub type MemSize = u64;

fn err_str<T: ToString>(err: T) -> String {
  err.to_string()
}

#[derive(Debug)]
pub struct Proc {
  pub stat: Box<ProcStat>,
  pub status: Box<ProcStatus>,
  pub cmdline: Vec<String>
}

impl Proc {
  pub fn new(pid: TaskId) -> Result<Self, ProcError> {
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

  fn read_cmdline(proc_dir: &str) -> Result<Vec<String>, ProcError> {
    File::open(Path::new(proc_dir).join("cmdline"))
      .or(Err(ProcSoftError(ProcReadError, ProcPartCmdline)))
      .and_then(|mut file| {
        let mut contents = Vec::new();
        try!(
          file.read_to_end(&mut contents)
            .or(Err(ProcSoftError(ProcReadError, ProcPartCmdline)))
        );
        if contents.ends_with(&['\0' as u8]) {
          let _ = contents.pop();
        }
        Ok(contents)
      }).and_then(|contents| {
        String::from_utf8(contents)
          .or(Err(ProcSoftError(ProcParseError, ProcPartCmdline)))
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
          if proc_s_r.as_ref().unwrap_err().is_hard() {
            return Some(proc_s_r.map_err(err_str));
          } else {
            return None;
          }
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

impl ProcQuery {
  fn create_query(query: &str) -> Result<ProcQuery, String> {
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
}

impl FromStr for ProcQuery {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::create_query(s)
  }
}

pub fn taskid_query(tid: TaskId, query: TaskId) -> bool {
  tid == query
}

pub fn string_query(text: &str, query: &str) -> bool {
  text.contains(query)
}
