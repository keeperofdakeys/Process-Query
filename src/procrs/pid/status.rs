use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::FromStr;
use ::error::{ProcError, ProcFile, ProcOper};
use ::{TaskId, MemSize};

#[derive(Debug)]
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
    pub fn new(pid_dir: &str) -> Result<Self, ProcError> {
        // Try opening file
        let status_file = try!(
            File::open(Path::new(pid_dir).join("status"))
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
                                             Some("No colon on line")))
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
