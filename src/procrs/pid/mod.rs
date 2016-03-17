use std::io;
use std::io::prelude::*;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::Path;
use std::vec;
use std::io::BufReader;
use std::cmp::Ordering;
use std::str::FromStr;

/// Get process stats (/proc/[pid]/stat)
pub mod stat;
/// Get process status (/proc/[pid]/status)
pub mod status;

use self::stat::PidStat;
use self::status::PidStatus;
use error::{ProcError, ProcFile, ProcOper};
use TaskId;

/// A struct containing information about a process.
///
/// This struct contains information from various files inside the
/// /proc/[pid] directory (for the respective pid).
#[derive(Debug)]
pub struct Pid {
    // FIXME: Take Vec<PidFile> to indicate which things to parse
    /// The tid of this process
    pub pid: TaskId,
    /// The /proc/[pid]/stat file
    pub stat: Box<PidStat>,
    /// The /proc/[pid]/status file
    pub status: Box<PidStatus>,
    /// The /proc/[pid]/cmdline file
    pub cmdline: Vec<String>,
    /// If this is a thread, this is set to true.
    /// Threads will never have tasks attached.
    is_thread: bool,
    /// Vec of threads under /proc/[pid]/tasks/[tid]
    threads: Option<Vec<Pid>>,
}

impl Pid {
    /// Create a new Pid struct for a process, given a pid.
    pub fn new(pid: TaskId) -> Result<Self, ProcError> {
        let pid_dir = Path::new("/proc");
        Self::new_dir(pid_dir, pid)
    }

    fn new_dir(proc_dir: &Path, pid: TaskId) -> Result<Self, ProcError> {
        let proc_dir = proc_dir.join(pid.to_string());
        let pid_stat = try!(PidStat::new(&proc_dir));
        let pid_status = try!(PidStatus::new(&proc_dir));
        let cmdline = try!(Self::read_cmdline(&proc_dir));

        Ok(Pid {
            pid: pid,
            stat: Box::new(pid_stat),
            status: Box::new(pid_status),
            cmdline: cmdline,
            is_thread: false,
            threads: None,
        })
    }

    /// Given a /proc/[pid] directory, read the respective /proc/[pid]/cmdline
    /// file and return them in a Vec.
    fn read_cmdline(proc_dir: &Path) -> Result<Vec<String>, ProcError> {
        File::open(proc_dir.join("cmdline"))
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

    pub fn tasks(&mut self) -> Option<Vec<Pid>> {
        self.tasks_query(PidQuery::NoneQuery)
    }

    // TODO: Work out if this really should return Option<_>
    // or Option<Result<Vec<Pid>>>. Otherwise the error is uncaught.
    pub fn tasks_query(&self, query: PidQuery) -> Option<Vec<Pid>> {
        if self.is_thread {
            return None;
        }

        PidIter::new_tid_query(self.pid, query.clone()).unwrap()
            .filter(|p| {
                let query = query.clone();
                match *p {
                    Ok(ref pid) => pid.query(&query),
                    Err(_) => true
                }
            }).collect::<Result<Vec<_>, _>>().ok()
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

/// A list of files in the pid directory.
pub enum PidFile {
    PidStatus,
    PidStat,
    PidCmdline
}

/// An Iterator over processes in the system.
///
/// If a process disappears while scanning it, the partial Pid struct
/// will not be yielded. An atomic view of processes on the system seems
/// non-trivial.
pub struct PidIter {
    dir: String,
    dir_iter: ReadDir,
    query: PidQuery,
}

impl PidIter {
    /// Create a new iterator over all processes in /proc.
    pub fn new() -> Result<Self, ProcError> {
        Self::new_query(PidQuery::NoneQuery)
    }

    /// Create a new iterator over all processes in /proc, but only yield
    /// processes that match the given query.
    pub fn new_query(query: PidQuery) -> Result<Self, ProcError> {
        let dir_name = "/proc".to_owned();
        let proc_dir = Path::new(&dir_name);
        let dir_iter = try!(
            fs::read_dir(proc_dir)
                .map_err(|e|
                    ProcError::new(ProcOper::Opening, ProcFile::ProcDir, Some(e), Some("PidIter"))
                )
        );
        Ok(PidIter {
            dir: dir_name.clone(),
            dir_iter: dir_iter,
            query: query,
        })
    }

    fn new_tid_query(pid: TaskId, query: PidQuery) -> Result<Self, ProcError> {
        let dir_name = format!("/proc/{}/task", pid);
        let task_dir = Path::new(&dir_name);
        let dir_iter = try!(
            fs::read_dir(task_dir)
                .map_err(|e|
                    ProcError::new(ProcOper::Opening, ProcFile::PidTaskDir,
                        Some(e), Some("PidIter"))
                )
        );
        Ok(PidIter {
            dir: dir_name.clone(),
            dir_iter: dir_iter,
            query: query
        })
    }

    /// Given a DirEntry, try to create a Pid struct, and only return if
    /// it matches the query, and is complete.
    fn proc_dir_filter(entry_opt: Result<DirEntry, io::Error>, query: &PidQuery, dir_name: &str)
        -> Option<Result<Pid, ProcError>> {
        let file = entry_opt
            .map_err(|e|
                ProcError::new(ProcOper::Reading, ProcFile::ProcDir, Some(e), Some("PidIter"))
            )
            .and_then(|entry|
                entry.file_name().into_string()
                    .or(Err(ProcError::new_more(ProcOper::Parsing, ProcFile::ProcDir, Some("PidIter"))))
            );

        if let Err(e) = file{
            return Some(Err(e));
        }

        // Ensure filename is an integer (skip if not)
        match file.unwrap().parse() {
            Ok(pid) => {
                // If an error is not hard (error opening or reading file),
                // do not error as it may be a now-dead process.
                // If a parsing error occurs, then do return an error.
                let prc = match Pid::new_dir(Path::new(&dir_name), pid) {
                    Ok(prc) => prc,
                    Err(e) => {
                        if e.is_hard() {
                            return Some(Err(e));
                        } else {
                            return None;
                        }
                    }
                };
                match prc.query(&query) {
                    true => Some(Ok(prc)),
                    false => None
                }
            },
            Err(_) => None
        }
    }
}

impl Iterator for PidIter {
    type Item = Result<Pid, ProcError>;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.dir_iter.by_ref() {
            match Self::proc_dir_filter(entry, &self.query, &self.dir) {
                some @ Some(_) => return some,
                None => continue
            }
        }
        None
    }

    /// Size may be anywhere from 0 to number of dirs.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.dir_iter.size_hint().1)
    }
}

/// An Iterator over threads of processes in the system.
///
/// If a task disappears while scanning it, the partial Pid struct
/// will not be yielded. An atomic view of processes on the system seems
/// non-trivial.
pub struct TidIter {
    pid_iter: PidIter,
    task_iter: Option<vec::IntoIter<Pid>>,
    query: PidQuery,
}

impl TidIter {
    /// Create a new iterator over all tasks in /proc.
    pub fn new() -> Result<Self, ProcError> {
            println!("{:?}", 3);
        Self::new_query(PidQuery::NoneQuery)
    }

    /// Create a new iterator over all tasks in /proc, but only yield
    /// those that match the given query.
    pub fn new_query(query: PidQuery) -> Result<Self, ProcError> {
        Ok(TidIter{
            pid_iter: try!(PidIter::new_query(query.clone())),
            task_iter: None,
            query: query,
        })
    }
}

impl Iterator for TidIter {
    type Item = Result<Pid, ProcError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.task_iter.is_none() {
                let pid = match self.pid_iter.next() {
                    Some(Ok(pid)) => pid,
                    Some(Err(e)) => { return Some(Err(e)) },
                    None => { return None; }
                };
                let tasks_vec = pid.tasks_query(self.query.clone());
                if let Some(vec) = tasks_vec {
                    self.task_iter = Some(vec.into_iter());
                }
                continue;
            } else {
                let next = self.task_iter.as_mut().unwrap().next();
                match next {
                    Some(pid) => { return Some(Ok(pid)); },
                    None => { self.task_iter = None; },
                };
            }
        }
    }
}

#[derive(Clone, Debug)]
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
