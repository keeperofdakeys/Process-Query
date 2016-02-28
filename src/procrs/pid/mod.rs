use std::io;
use std::io::prelude::*;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::io::BufReader;
use std::cmp::Ordering;
use std::str::FromStr;

/// Get process stats (/proc/[pid]/stat)
mod stat;
/// Get process status (/proc/[pid]/status)
mod status;

use self::stat::PidStat;
use self::status::PidStatus;
use error::{ProcError, ProcFile, ProcOper};
use TaskId;

fn err_str<T: ToString>(err: T) -> String {
    err.to_string()
}

/// A struct containing information about a process.
///
/// This struct contains information from various files inside the
/// /proc/[pid] directory (for the respective pid).
#[derive(Debug)]
pub struct Pid {
    /// The /proc/[pid]/stat file
    pub stat: Box<PidStat>,
    /// The /proc/[pid]/status file
    pub status: Box<PidStatus>,
    /// The /proc/[pid]/cmdline file
    pub cmdline: Vec<String>
}

impl Pid {
    pub fn new(pid: TaskId) -> Result<Self, ProcError> {
        let proc_dir = format!("/proc/{}", pid);
        let pid_stat = try!(PidStat::new(&proc_dir));
        let pid_status = try!(PidStatus::new(&proc_dir));
        let cmdline = try!(Self::read_cmdline(&proc_dir));

        Ok(Pid{
            stat: Box::new(pid_stat),
            status: Box::new(pid_status),
            cmdline: cmdline
        })
    }

    /// Given a /proc/[pid] directory, read the respective /proc/[pid]/cmdline
    /// file and return them in a Vec.
    fn read_cmdline(proc_dir: &str) -> Result<Vec<String>, ProcError> {
        File::open(Path::new(proc_dir).join("cmdline"))
            .map_err(|e| ProcError::new_err(ProcOper::Opening, ProcFile::PidCmdline, e))
            .and_then(|file| {
                let mut contents = Vec::new();
                try!(
                    BufReader::with_capacity(4096, file)
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

    /// Determine whether this process matches this query
    fn query(&self, query: &PidQuery) -> bool {
        match *query {
            PidQuery::PidQuery(q) => PidQuery::taskid_query(self.stat.pid, q),
            PidQuery::PpidQuery(q) => PidQuery::taskid_query(self.stat.ppid, q),
            PidQuery::NameQuery(ref q) => PidQuery::string_query(&self.stat.comm, &q),
            PidQuery::CmdlineQuery(ref q) => PidQuery::string_query(&self.cmdline.join(" "), &q),
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

/// An Iterator over processes in the system.
///
/// If a process disappears while scanning it, the partial Pid struct
/// will not be yielded. An atomic view of processes on the system seems
/// non-trivial.
pub struct PidIter {
    dir_iter: ReadDir,
    query: PidQuery
}

impl PidIter {
    /// Create a new iterator over all processes in /proc.
    pub fn new() -> Result<Self, String> {
        Self::new_query(PidQuery::NoneQuery)
    }

    /// Create a new iterator over all processes in /proc, but only yield
    /// processes that match the given query.
    pub fn new_query(query: PidQuery) -> Result<Self, String> {
        let proc_dir = Path::new("/proc");
        let dir_iter = try!(fs::read_dir(proc_dir).map_err(err_str));
        Ok(PidIter{
            dir_iter: dir_iter,
            query: query
        })
    }

    /// Given a DirEntry, try to create a Pid struct, and only return if
    /// it matches the query, and is complete.
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
                // If an error is not hard (error opening or reading file),
                // do not error as it may be a now-dead process.
                // If a parsing error occurs, then do return an error.
                let prc = match Pid::new(pid) {
                    Ok(prc) => prc,
                    Err(e) => {
                        if e.is_hard() {
                            return Some(Err(e).map_err(err_str));
                        } else {
                            return None;
                        }
                    }
                };
                match prc.query(query) {
                    true => Some(Ok(prc)),
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

/// A list of query types for process querying.
pub enum PidQuery {
    /// Query by pid
    PidQuery(TaskId),
    /// Query by ppid
    PpidQuery(TaskId),
    /// Query by program name
    NameQuery(String),
    /// Query by cmdline contents (joined with space)
    CmdlineQuery(String),
    /// An empty query that always matches
    NoneQuery
}

impl PidQuery {
    /// Given a user-specified query string, decode it into
    /// an appropriate query.
    ///
    /// Bare number -> PidQuery
    /// Bare string -> NameQuery
    ///
    /// type=query is supported for the following types;
    /// pid -> PidQuery
    /// ppid -> PpidQuery
    /// name -> NameQuery
    /// cmdline -> CmdlineQuery
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

    /// Match a pid by simple equality.
    pub fn taskid_query(tid: TaskId, query: TaskId) -> bool {
        tid == query
    }

    /// For strings, use a substring search.
    pub fn string_query(text: &str, query: &str) -> bool {
        text.contains(query)
    }
}

impl FromStr for PidQuery {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::create_query(s)
    }
}
