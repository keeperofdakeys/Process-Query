use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::FromStr;
use ::error::{ProcError, ProcFile, ProcOper};
use ::{TaskId, MemSize};

#[derive(Debug, PartialEq)]
pub struct PidStatus {
    pub name: String,
    pub tgid: TaskId,
    pub pid: TaskId,
    pub ppid: TaskId,
    pub tracerpid: TaskId,
    // uid: Real, Effective, Saved, Filesystem
    pub uid: (u32, u32, u32, u32),
    // gid: Real, Effective, Saved, Filesystem
    pub gid: (u32, u32, u32, u32),
    pub fdsize: u32,
    pub vmpeak: Option<MemSize>,
    pub vmsize: Option<MemSize>,
    pub vmlck: Option<MemSize>,
    pub vmpin: Option<MemSize>,
    pub vmhwm: Option<MemSize>,
    pub vmrss: Option<MemSize>,
    pub vmdata: Option<MemSize>,
    pub vmstk: Option<MemSize>,
    pub vmexe: Option<MemSize>,
    pub vmlib: Option<MemSize>,
    pub vmpte: Option<MemSize>,
    pub vmpmd: Option<MemSize>,
    pub vmswap: Option<MemSize>,
    pub threads: u32
}

// Try removing key, if it's missing throw an error.
// Then try parsing it, then returning a final value if no errors have occured.
macro_rules! extract_line {
    ($map:expr, $key:expr, $func:expr) =>
        (try!(
            $map.remove($key)
                .ok_or(ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus,
                    Some(concat!("missing ", $key)))
                ).and_then(|s|
                    $func(s).map_err(|e|
                        ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
                            Some(e), Some($key))
                    )
                )
        ))
}

// Similar to extract_line, except that a missing field isn't an error.
macro_rules! extract_line_opt {
    ($map:expr, $key:expr, $func:expr) =>
        (match $map.remove($key) {
            Some(s) => Some(try!(
                $func(s).map_err(|e|
                    ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
                        Some(e), Some($key))
                )
            )),
            None => None
        })
}

impl PidStatus {
    // Generate PidStatus struct given a process directory
    pub fn new(pid_dir: &Path) -> Result<Self, ProcError> {
        // Try opening file
        let status_file = try!(
            File::open(pid_dir.join("status"))
                .map_err(|e| ProcError::new_err(ProcOper::Opening, ProcFile::PidStatus, e))
        );

        let lines =
            BufReader::with_capacity(4096, status_file)
                .lines()
                .map(|r|
                    match r {
                        Ok(o) => Ok(o),
                        Err(e) => Err(ProcError::new_err(ProcOper::Reading, ProcFile::PidStatus, e))
                    }
                );
        Self::parse_string(lines)
    }

    fn parse_string<I: Iterator<Item=Result<String, ProcError>>>(lines: I) -> Result<Self, ProcError> {
        let mut status: HashMap<_, _> = 
            try!(
                lines.map(|r|
                    match r {
                        Ok(line) => {
                            let split = line.splitn(2, ':').collect::<Vec<&str>>();
                            match (split.get(0), split.get(1)) {
                                (Some(key), Some(value)) =>
                                    Ok((key.trim().to_owned(),
                                         value.trim().to_owned())),
                                _ => Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStatus,
                                             Some("Line missing colon")))
                            }
                        },
                        Err(e) => Err(e)
                    }
                ).collect::<Result<_, _>>());


        // It's quite important that these appear in the order that they
        // appear in the status file
        Ok(PidStatus{
            name: extract_line!(status, "Name", |s| Ok(s) as Result<String, ProcError>),
            tgid: extract_line!(status, "Tgid", parse_any),
            pid: extract_line!(status, "Pid", parse_any),
            ppid: extract_line!(status, "PPid", parse_any),
            tracerpid: extract_line!(status, "TracerPid", parse_any),
            uid : extract_line!(status, "Uid", parse_uids),
            gid : extract_line!(status, "Gid", parse_uids),
            fdsize : extract_line!(status, "FDSize", parse_any),
            vmpeak: extract_line_opt!(status, "VmPeak", parse_mem),
            vmsize: extract_line_opt!(status, "VmSize", parse_mem),
            vmlck: extract_line_opt!(status, "VmLck", parse_mem),
            vmpin: extract_line_opt!(status, "VmPin", parse_mem),
            vmhwm: extract_line_opt!(status, "VmHWM", parse_mem),
            vmrss: extract_line_opt!(status, "VmRSS", parse_mem),
            vmdata: extract_line_opt!(status, "VmData", parse_mem),
            vmstk: extract_line_opt!(status, "VmStk", parse_mem),
            vmexe: extract_line_opt!(status, "VmExe", parse_mem),
            vmlib: extract_line_opt!(status, "VmLib", parse_mem),
            vmpmd: extract_line_opt!(status, "VmPMD", parse_mem),
            vmpte: extract_line_opt!(status, "VmPTE", parse_mem),
            vmswap: extract_line_opt!(status, "VmSwap", parse_mem),
            threads: extract_line!(status, "Threads", parse_any)
        })
    }
}

// Parse anything that's parsable (type checker didn't like simple closures).
fn parse_any<N: FromStr>(str: String) -> Result<N, N::Err> {
    str.parse()
}

fn parse_uids(uid_str: String) -> Result<(u32, u32, u32, u32), ProcError> {
    let uids = try!(
        uid_str.split("\t")
            .filter(|s| s != &"")
            .map(|s|
                s.parse()
            ).collect::<Result<Vec<_>, _>>()
            .map_err(|e|
                ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
                    Some(e), Some("parsing uid"))
            )

    );
    if uids.len() != 4 {
        return Err(ProcError::new_more(ProcOper::ParsingField,
            ProcFile::PidStatus, Some("missing uids")));
    }
    Ok((uids[0], uids[1], uids[2], uids[3]))
}

fn parse_mem(mem_str: String) -> Result<MemSize, ParseIntError> {
    mem_str.trim_right_matches(" kB")
        .parse::<MemSize>()
        .map(|n| n * 1024)
}

#[test]
fn test_no_colon() {
    let lines = "Name".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStatus, Some("Line missing colon")))
    );
}

#[test]
fn test_missing_tgid() {
    let lines = "Name: a\n\
                 Pid: 4\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus, Some("missing Tgid")))
    );
}

#[test]
fn test_uid_parse() {
    let lines = "Name:	bash\n\
                 Tgid:	27899\n\
                 Ngid:	0\n\
                 Pid:	27899\n\
                 PPid:	4351\n\
                 TracerPid:	0\n\
                 Uid:	1000	1000	a000	1000\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
            Some("a".parse::<u8>().unwrap_err()), Some("Uid")))
    );
}

#[test]
fn test_uid_count() {
    let lines = "Name:	bash\n\
                 Tgid:	27899\n\
                 Ngid:	0\n\
                 Pid:	27899\n\
                 PPid:	4351\n\
                 TracerPid:	0\n\
                 Uid:	1000	1000	1000\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus, Some("Uid")))
    );
}

#[test]
fn test_mem_parse() {
    let lines = "Name:	bash\n\
                 Tgid:	27899\n\
                 Ngid:	0\n\
                 Pid:	27899\n\
                 PPid:	4351\n\
                 TracerPid:	0\n\
                 Uid:	1000	1000	1000	1000\n\
                 Gid:	1000	1000	1000	1000\n\
                 FDSize:	256\n\
                 Groups:	10 18 27 35 101 103 104 105 250 1000 1001 \n\
                 NStgid:	27899\n\
                 NSpid:	27899\n\
                 NSpgid:	27899\n\
                 NSsid:	27899\n\
                 VmPeak:	   a0896 kB\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
            Some("a".parse::<u8>().unwrap_err()), Some("VmPeak")))
    );
}

#[test]
fn test_parsing() {
    let lines = "Name:	bash\n\
                 Tgid:	27899\n\
                 Pid:	27899\n\
                 PPid:	4351\n\
                 TracerPid:	0\n\
                 Uid:	1000	1000	1000	1000\n\
                 Gid:	1000	1000	1000	1000\n\
                 FDSize:	256\n\
                 Groups:	10 18 27 35 101 103 104 105 250 1000 1001 \n\
                 NStgid:	27899\n\
                 NSpid:	27899\n\
                 NSpgid:	27899\n\
                 NSsid:	27899\n\
                 VmPeak:	   20896 kB\n\
                 VmSize:	   20868 kB\n\
                 VmLck:	       0 kB\n\
                 VmPin:	       0 kB\n\
                 VmHWM:	    4584 kB\n\
                 VmRSS:	    4584 kB\n\
                 VmData:	    1176 kB\n\
                 VmStk:	     136 kB\n\
                 VmExe:	     688 kB\n\
                 VmLib:	    2540 kB\n\
                 VmPTE:	      64 kB\n\
                 VmPMD:	      12 kB\n\
                 VmSwap:	       0 kB\n\
                 Threads:	1\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Ok(PidStatus {
            name: "bash".to_owned(),
            tgid: 27899,
            pid: 27899,
            ppid: 4351,
            tracerpid: 0,
            uid: (1000, 1000, 1000, 1000),
            gid: (1000, 1000, 1000, 1000),
            fdsize: 256,
            vmpeak: Some(21397504),
            vmsize: Some(21368832),
            vmlck: Some(0),
            vmpin: Some(0),
            vmhwm: Some(4694016),
            vmrss: Some(4694016),
            vmdata: Some(1204224),
            vmstk: Some(139264),
            vmexe: Some(704512),
            vmlib: Some(2600960),
            vmpte: Some(65536),
            vmpmd: Some(12288),
            vmswap: Some(0),
            threads: 1
        })
    );
}
