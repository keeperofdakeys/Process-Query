use std::str::FromStr;
use procrs::pid::PidFile;

// FIXME: This may be better in procps
enum PidCols {
    Pid,
    Ppid
}

impl PidCols {
    fn get_file(&self) -> PidFile {
        match *self {
            PidCols::Pid => PidFile::PidStat,
            PidCols::Ppid => PidFile::PidStat,
        }
    }
    // fn get_files(cols: &[PidCols]) -> HashSet<PidCols> {
    //     let mut files = HashSet::new();
    //     for col in cols {
    //         files.insert(
    //     }
    // }
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
