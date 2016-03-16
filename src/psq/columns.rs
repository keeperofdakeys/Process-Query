use std::str::FromStr;

// FIXME: This may be better in procps
enum PidCols {
    Pid,
    Ppid
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
