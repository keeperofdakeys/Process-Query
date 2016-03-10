use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::num::ParseIntError;
use ::error::{ProcError, ProcFile, ProcOper};
use ::{TaskId, MemSize};

/// Parse a line, by turning a parsing error into a ProcError
macro_rules! parse {
    ($value: expr, $key: expr) => {
        Some(try!(
        $value.map_err(|e|
            ProcError::new(ProcOper::ParsingField, ProcFile::PidStatus,
                Some(e), Some($key))
        )))
    }
}

/// Unwrap a line, emitting a "missing '$key'" ProcError if None
macro_rules! unwrap {
    ($value: expr, $key: expr) => {
        try!(
        $value.ok_or(
            ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus,
                Some(concat!("missing ", $key)))
        ))
    }
}

#[derive(Debug, PartialEq)]
/// A struct containing information from the status file for a process.
///
/// This struct contains information from the /proc/[pid]/status or
/// /proc/[tgid]/task/[tid]/status file, for a specific pid or tgid/tid.
pub struct PidStatus {
    // TODO: Maybe these should all be optional, and be more annoying to call
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

impl PidStatus {
    /// Generate PidStatus struct given a process directory
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

    /// Parse an Iterator of lines as a /proc/[pid]/status file.
    fn parse_string<I: Iterator<Item=Result<String, ProcError>>>(lines: I) -> Result<Self, ProcError> {
        let (mut name, mut tgid, mut pid, mut ppid, mut tracerpid, mut uid,
            mut gid, mut fdsize, mut vmpeak, mut vmsize, mut vmlck, mut vmpin,
            mut vmhwm, mut vmrss, mut vmdata, mut vmstk, mut vmexe, mut vmlib,
            mut vmpte, mut vmpmd, mut vmswap, mut threads) =
            (None, None, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None, None, None);
        for line in lines {
            let line = try!(line);
            // Find colon offset, error on no match.
            let colon_offset = match line.find(':') {
                Some(i) => i,
                None => return Err(
                    ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus, Some("Line missing colon"))
                ),
            };
            // Split into Key: Value based on colon offset.
            let (first, second) = line.split_at(colon_offset);
            let key = first.trim();
            let (_, last) = second.split_at(1);
            let value = last.trim();

            match key {
                "Name" => name = parse!(Ok(value.to_owned()) as Result<String, ProcError>, "Name"),
                "Tgid" => tgid = parse!(value.parse(), "Tgid"),
                "Pid" => pid = parse!(value.parse(), "Pid"),
                "PPid" => ppid = parse!(value.parse(), "PPid"),
                "TracerPid" => tracerpid = parse!(value.parse(), "TracerPid"),
                "Uid" => uid  = parse!(parse_uids(value), "Uid"),
                "Gid" => gid  = parse!(parse_uids(value), "Gid"),
                "FDSize" => fdsize  = parse!(value.parse(), "FDSize"),
                "VmPeak" => vmpeak = parse!(parse_mem(value), "VmPeak"),
                "VmSize" => vmsize = parse!(parse_mem(value), "VmSize"),
                "VmLck" => vmlck = parse!(parse_mem(value), "VmLck"),
                "VmPin" => vmpin = parse!(parse_mem(value), "VmPin"),
                "VmHWM" => vmhwm = parse!(parse_mem(value), "VmHWM"),
                "VmRSS" => vmrss = parse!(parse_mem(value), "VmRSS"),
                "VmData" => vmdata = parse!(parse_mem(value), "VmData"),
                "VmStk" => vmstk = parse!(parse_mem(value), "VmStk"),
                "VmExe" => vmexe = parse!(parse_mem(value), "VmExe"),
                "VmLib" => vmlib = parse!(parse_mem(value), "VmLib"),
                "VmPTE" => vmpte = parse!(parse_mem(value), "VmPTE"),
                "VmPMD" => vmpmd = parse!(parse_mem(value), "VmPMD"),
                "VmSwap" => vmswap = parse!(parse_mem(value), "VmSwap"),
                "Threads" => threads = parse!(value.parse(), "Threads"),
                _ => continue,
            };
        }
        Ok(PidStatus {
            name: unwrap!(name, "Name"),
            tgid: unwrap!(tgid, "Tgid"),
            pid: unwrap!(pid, "Pid"),
            ppid: unwrap!(ppid, "PPid"),
            tracerpid: unwrap!(tracerpid, "TracerPid"),
            uid: unwrap!(uid, "Uid"),
            gid: unwrap!(gid, "Gid"),
            fdsize: unwrap!(fdsize, "FDSize"),
            vmpeak: vmpeak,
            vmsize: vmsize,
            vmlck: vmlck,
            vmpin: vmpin,
            vmhwm: vmhwm,
            vmrss: vmrss,
            vmdata: vmdata,
            vmstk: vmstk,
            vmexe: vmexe,
            vmlib: vmlib,
            vmpte: vmpte,
            vmpmd: vmpmd,
            vmswap: vmswap,
            threads: unwrap!(threads, "Threads"),
        })
    }
}

/// Parse a set of four numbers as uids or gids.
fn parse_uids(uid_str: &str) -> Result<(u32, u32, u32, u32), ProcError> {
    let uids = try!(
        uid_str.split_whitespace()
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

/// Parse a string as a kB memory string.
fn parse_mem(mem_str: &str) -> Result<MemSize, ParseIntError> {
    mem_str.trim_right_matches(" kB")
        .parse::<MemSize>()
        .map(|n| n * 1024)
}

#[test]
fn test_no_colon() {
    let lines = "Name".lines().map(|l| Ok(l.to_owned()));
    let status = PidStatus::parse_string(lines);
    assert_eq!(status,
        Err(ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStatus, Some("Line missing colon")))
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
fn test_optional_parse() {
    let lines = "Name:  kthreadd\n\
                 State: S (sleeping)\n\
                 Tgid:  2\n\
                 Ngid:  0\n\
                 Pid:   2\n\
                 PPid:  0\n\
                 TracerPid: 0\n\
                 Uid:   0   0   0   0\n\
                 Gid:   0   0   0   0\n\
                 FDSize:    64\n\
                 Groups:    \n\
                 NStgid:    2\n\
                 NSpid: 2\n\
                 NSpgid:    0\n\
                 NSsid: 0\n\
                 Threads:   1\n\
                 ".lines().map(|l| Ok(l.to_owned()));
    let _ = PidStatus::parse_string(lines).unwrap();
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
