use std::str::FromStr;
use std::iter::IntoIterator;
use std::collections::HashSet;
use procrs::pid::PidFile;

// FIXME: This may be better in procps
enum PidCols {
    Pid,
    Tid,
    Tgid,
    Ppid,
    RSS,
    Time,
    Cmd,
    CmdLine
}

impl PidCols {
    fn get_file(&self) -> PidFile {
        match *self {
            PidCols::Pid => PidFile::PidStat,
            PidCols::Tid => PidFile::PidStat,
            PidCols::Ppid => PidFile::PidStat,
            PidCols::Tgid => PidFile::PidStatus,
            PidCols::RSS => PidFile::PidStatus,
            PidCols::Time => PidFile::PidStatus,
            PidCols::Cmd => PidFile::PidStat,
            PidCols::CmdLine => PidFile::PidCmdline
        }
    }
}

// Implement FromStr to allow parsing a list of columns specified by a user
impl FromStr for PidCols {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pid" => PidCols::Pid,
            "ppid" => PidCols::Ppid,
            _ => return Err(()),
        })
    }
}


// Implement ToStr to get a list of columns specified by a user
// impl ToStr for PidColumns {
// }
