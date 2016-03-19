use std::str::FromStr;
use std::iter::IntoIterator;
use std::collections::HashSet;
use procrs::pid::PidFile;

// FIXME: This may be better in procps
enum PidCol {
    Pid,
    Tid,
    Tgid,
    Ppid,
    RSS,
    Time,
    Cmd,
    Cmdline
}

impl PidCol {
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

    fn to_str(s: PidCol) -> Result<&'static str, ()> {
        Ok(match s {
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
}

// Implement FromStr to allow parsing a list of columns specified by a user
impl FromStr for PidCol {
    type Err = ();

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
