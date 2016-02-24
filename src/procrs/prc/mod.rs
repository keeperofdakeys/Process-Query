use std::io;
use std::io::prelude::*;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::io::BufReader;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::str::FromStr;

mod stat;
mod status;

use self::stat::PidStat;
use self::status::PidStatus;
use ::error::{ProcError, ProcFile, ProcOper};

pub type TaskId = i32;
pub type MemSize = u64;

fn err_str<T: ToString>(err: T) -> String {
  err.to_string()
}

#[derive(Debug)]
pub struct Pid {
  pub stat: Box<PidStat>,
  pub status: Box<PidStatus>,
  pub cmdline: Vec<String>
}

impl Pid {
  pub fn new(pid: TaskId) -> Result<Self, ProcError> {
    let proc_dir = format!("/proc/{}", pid);
    let proc_stat = try!(PidStat::new(&proc_dir));
    let proc_status = try!(PidStatus::new(&proc_dir));
    let cmdline = try!(Self::read_cmdline(&proc_dir));

    let proc_struct = Pid{
      stat: Box::new(proc_stat),
      status: Box::new(proc_status),
      cmdline: cmdline
    };

    Ok(proc_struct)
  }

  fn read_cmdline(proc_dir: &str) -> Result<Vec<String>, ProcError> {
    File::open(Path::new(proc_dir).join("cmdline"))
      .map_err(|e| ProcError::new_err(ProcOper::Opening, ProcFile::PidCmdline, e))
      .and_then(|file| {
        let mut contents = Vec::new();
        try!(
          BufReader::new(file)
            .read_to_end(&mut contents)
            .map_err(|e| ProcError::new_err(ProcOper::Reading, ProcFile::PidCmdline, e))
        );
        if contents.ends_with(&['\0' as u8]) {
          let _ = contents.pop();
        }
        Ok(contents)
      }).and_then(|contents| {
        String::from_utf8(contents)
          .or(Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidCmdline,
                  Some("parsing utf8"))))
      }).map(|contents|
        contents
          .split('\0')
          .map(|a| a.to_string())
          .collect()
      )
  }

  // Return true if query matches this process
  fn query(&self, query: &PidQuery) -> bool {
    match *query {
      PidQuery::PidQuery(q) => taskid_query(self.stat.pid, q),
      PidQuery::PpidQuery(q) => taskid_query(self.stat.ppid, q),
      PidQuery::NameQuery(ref q) => string_query(&self.stat.comm, &q),
      PidQuery::CmdlineQuery(ref q) => string_query(&self.cmdline.join(" "), &q),
      PidQuery::NoneQuery => true
    }
  }
}

impl PartialEq for Pid {
  fn eq(&self, other: &Self) -> bool {
    self.stat.pid.eq(&other.stat.pid)
  }
}

impl Eq for Pid {}
impl PartialOrd for Pid {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
impl Ord for Pid {
  fn cmp(&self, other: &Self) -> Ordering {
    self.stat.pid.cmp(&other.stat.pid)
  }
}

pub struct PidIter {
  dir_iter: ReadDir,
  query: PidQuery
}

impl PidIter {
  pub fn new() -> Result<Self, String> {
    Self::new_query(PidQuery::NoneQuery)
  }

  pub fn new_query(query: PidQuery) -> Result<Self, String> {
    let proc_dir = Path::new("/proc");
    let dir_iter = try!(fs::read_dir(proc_dir).map_err(err_str));
    Ok(PidIter{
      dir_iter: dir_iter,
      query: query
    })
  }

  fn proc_dir_filter(entry_opt: Result<DirEntry, io::Error>, query: &PidQuery)
    -> Option<Result<Pid, String>> {
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
        let proc_s_r = Pid::new(pid);
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

impl Iterator for PidIter {
  type Item = Result<Pid, String>;

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

pub type PidMap = HashMap<TaskId, Pid>;

pub fn get_proc_map() -> Result<PidMap, String> {
  let iter = try!(PidIter::new());
  iter.map(|proc_s|
    proc_s.map(|p|
      (p.stat.pid, p)
    )
  ).collect()
}

pub enum PidQuery {
  PidQuery(TaskId),
  PpidQuery(TaskId),
  NameQuery(String),
  CmdlineQuery(String),
  NoneQuery
}

impl PidQuery {
  fn create_query(query: &str) -> Result<PidQuery, String> {
    let splits: Vec<_> = query.splitn(2, '=').collect();

    match splits.len() {
      0 => Ok(PidQuery::NoneQuery),
      1 => Ok(match query.parse().ok() {
        Some(tid) => PidQuery::PidQuery(tid),
        None => PidQuery::NameQuery(query.to_owned())
      }),
      _ => {
        let q_text = splits[1].to_owned();
        let q_tid = q_text.parse();
        match &*splits[0].to_lowercase() {
          "pid" => q_tid.map(|q| PidQuery::PidQuery(q))
            .or(Err("Query value for type 'pid' not valid".to_owned())),
          "ppid" => q_tid.map(|q| PidQuery::PpidQuery(q))
            .or(Err("Query value for type 'ppid' not valid".to_owned())),
          "name" => Ok(PidQuery::NameQuery(q_text)),
          "cmdline" => Ok(PidQuery::CmdlineQuery(q_text)),
          _ => Err("Invalid query type".to_owned())
        }
      }
    }
  }
}

impl FromStr for PidQuery {
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
