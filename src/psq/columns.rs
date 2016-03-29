use std::str::FromStr;
use std::iter::IntoIterator;
use std::collections::HashSet;
use procrs::pid::{PidFile, Pid};

// FIXME: This may be better in procps
enum PidCol {
    /// Process ID
    Pid,
    /// Thread ID (kernel's Pid)
    Tid,
    /// Thread Group ID
    Tgid,
    /// Parent Process ID
    Ppid,
    /// Resident Memory
    RSS,
    /// CPU Time
    Time,
    /// Process Name
    Cmd,
    /// Process Arguments
    Cmdline
}

impl PidCol {
    /// Get the file that this column requires.
    fn get_file(&self) -> PidFile {
        match *self {
            PidCol::Pid => PidFile::PidStat,
            PidCol::Tid => PidFile::PidStat,
            PidCol::Ppid => PidFile::PidStat,
            PidCol::Tgid => PidFile::PidStatus,
            PidCol::RSS => PidFile::PidStatus,
            PidCol::Time => PidFile::PidStatus,
            PidCol::Cmd => PidFile::PidStat,
            PidCol::Cmdline => PidFile::PidCmdline
        }
    }

    /// Get the str of this column.
    fn to_str(&self) -> Result<&'static str, ()> {
        Ok(match *self {
            PidCol::Pid => "pid",
            PidCol::Tid => "tid",
            PidCol::Ppid => "ppid",
            PidCol::Tgid => "tgid",
            PidCol::RSS => "rss",
            PidCol::Time => "time",
            PidCol::Cmd => "cmd",
            PidCol::Cmdline => "cmdline",
        })
    }

    /// Get the title of this column>
    fn to_title(&self) -> Result<&'static str, ()> {
        Ok(match *self {
            PidCol::Pid => "Pid",
            PidCol::Tid => "Iid",
            PidCol::Ppid => "Ppid",
            PidCol::Tgid => "Tgid",
            PidCol::RSS => "RSS",
            PidCol::Time => "Time",
            PidCol::Cmd => "Cmd",
            PidCol::Cmdline => "Cmdline",
        })
    }

    /// Get the set of files that some list of columns require.
    fn get_file_set<I: IntoIterator<Item=PidCol>>(cols_iter: I) -> HashSet<PidFile> {
        cols_iter.into_iter()
            .map(|pid_col| pid_col.get_file())
            .collect()
    }
}

// Implement FromStr to allow parsing a list of columns specified by a user
impl FromStr for PidCol {
    type Err = ();

    /// Get the column for a given column str.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pid" => PidCol::Pid,
            "tid" => PidCol::Tid,
            "ppid" => PidCol::Ppid,
            "tgid" => PidCol::Tgid,
            "rss" => PidCol::RSS,
            "time" => PidCol::Time,
            "cmd" => PidCol::Cmd,
            "cmdline" => PidCol::Cmdline,
            _ => return Err(()),
        })
    }
}

fn create_titles(cols: &[PidCol]) -> Vec<String> {
  cols.iter().map(|c| {
    c.to_title().unwrap().to_owned()
  }).collect()
}

fn create_row(cols: &[PidCol], pid: Pid) -> Vec<String> {
  cols.iter().map(|c| {
    match c.to_str() {
      _ => unimplemented!()
    }
  }).collect()
}
