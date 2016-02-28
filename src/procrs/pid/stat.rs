use std::fs::File;
use std::path::Path;
use std::io::{Read, BufReader};
use error::{ProcError, ProcFile, ProcOper};
use TaskId;

/// A struct containing information from the stat file for a process.
///
/// This struct contains information from the /proc/[pid]/stat file
/// for a specific pid.
#[derive(Debug, Clone, PartialEq)]
pub struct PidStat {
    /// The process id.
    pub pid: TaskId,
    /// The filename of the executable.
    pub comm: String,
    /// The process state.
    pub state: PidState,
    /// The process id of the parent process.
    pub ppid: TaskId,
    /// The process group id.
    pub pgrp: i32,
    /// The session id of this process.
    pub session: i32,
    /// The controlling tty of this process.
    pub tty_nr: i32,
    /// The id of the process controlling the tty of this process.
    pub tpgid: i32,
    /// The kernel flags of this processj.
    pub flags: u32,
    /// Count of minor page faults not requiring disk access.
    pub minflt: u64,
    /// Count of minor page faults in children we are waiting for.
    pub cminflt: u64,
    /// Count of major page faults not requiring disk acceess.
    pub majflt: u64,
    /// Count of major page faults in children we are waiting for.
    pub cmajflt: u64,
    /// Amout of time this process has been scheduled in user mode.
    pub utime: u64,
    /// Amount of time this process has been scheduled in kernel mode.
    pub stime: u64,
    /// Amount of time children we are waiting for have been scheduled in user mode.
    pub cutime: i64,
    /// Amount of time children we are waiting for have been scheduled in kernel mode.
    pub cstime: i64,
    /// Priority of process.
    pub priority: i64,
    /// Process nice value (19 low -> -20 high).
    pub nice: i64,
    /// Number of threads this process is using.
    pub num_threads: i64,
    /// Count of jiffies before we receive the next SIGALRM (0 since kernel 2.6.17).
    pub itrealvalue: i64,
    /// The time the process started after boot (ticks since kernel 2.6).
    pub starttime: u64,
    /// Virtual memory size in bytes.
    pub vsize: u64,
    /// Resident set size in pages.
    pub rss: i64,
    /// RSS soft limit of process.
    pub rsslim: u64,
    /// Memory address where executable memory starts.
    pub startcode: u64,
    /// Memory address where executable memory stops.
    pub endcode: u64,
    /// The start address of the stack (ie: bottom).
    pub startstack: u64,
    /// The current value of ESP (stack pointer).
    pub kstkesp: u64,
    /// The current EIP (instruction pointer).
    pub kstkeip: u64,
    /// The bitmap of pending signals, displayed as a decimal number.
    pub signal: u64,
    /// The bitmap of blocked signals, displayed as a decimal number.
    pub blocked: u64,
    /// The bitmap of ignored signals, displayed as a decimal number.
    pub sigignore: u64,
    /// The bitmap of caught signals, displayed as a decimal number.
    pub sigcatch: u64,
    /// This is the "channel" in which the process is waiting.
    pub wchan: u64,
    /// Number of pages swapped (not maintained).
    pub nswap: u64,
    /// Cumulative nswap for child processes (not maintained).
    pub cnswap: u64,
    // These fields depend on kernel version (linux 2.1 -> 3.5), so wrap in Option.
    /// Signal sent to the parent when we die.
    pub exit_signal: Option<i32>,
    /// CPU number last executed on.
    pub processor: Option<i32>,
    /// Real-time scheduling priority.
    pub rt_priority: Option<u32>,
    /// Scheduling policy.
    pub policy: Option<u32>,
    /// Aggregated block I/O delays, measured in clock ticks.
    pub delayacct_blkio_ticks: Option<u64>,
    /// Time spent running a virtual CPU for a guest OS, in clock ticks.
    pub guest_time: Option<u64>,
    /// Guest time of the process's children, in clock ticks.
    pub cguest_time: Option<i64>,
    /// Address above which init'd and uninit'd (BSS) data is placed.
    pub start_data: Option<u64>,
    /// Address below which init'd and uninit'd (BSS) data is placed.
    pub end_data: Option<u64>,
    /// Address above which program heap can be expanded.
    pub start_brk: Option<u64>,
    /// Address above which program cmdline args (arv) are placed.
    pub arg_start: Option<u64>,
    /// Address below  which program cmdline args (arv) are placed.
    pub arg_end: Option<u64>,
    /// Address above which program environment is placed.
    pub env_start: Option<u64>,
    /// Address below which program environment is placed.
    pub env_end: Option<u64>,
    /// The thread's exit status.
    pub exit_code: Option<i32>
}

/// Macro to parse a number, replacing errors with PidError.
macro_rules! stat_parse_num {
    ($item:expr) =>
        (try!(
            $item.ok_or(
                ProcError::new_more(ProcOper::ParsingField, ProcFile::PidStat, Some("missing field"))
            ).and_then(|s|
                 s.parse()
                     .map_err(|e| ProcError::new(ProcOper::ParsingField, ProcFile::PidStat,
                                                    Some(e), Some("parsing number")))
            )
        ))
}

/// Macro to parse an optional number, replacing errors with PidError.
macro_rules! stat_parse_opt_num {
    ($item:expr) =>
        (match $item {
            Some(n) => Some(stat_parse_num!(Some(n))),
            None => None
        })
}

impl PidStat {
    /// Generate PidStat struct given a process directory.
    pub fn new(pid_dir: &str) -> Result<Self, ProcError> {
        let file = try!(
            File::open(Path::new(pid_dir).join("stat"))
                .map_err(|e|
                    ProcError::new_err(ProcOper::Opening, ProcFile::PidStat, e)
                )
        );
        let bytes = try!(BufReader::with_capacity(4096, file)
            .bytes().collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProcError::new_err(ProcOper::Reading, ProcFile::PidStat, e))
            .and_then(|s|
                String::from_utf8(s)
                .map_err(|e| ProcError::new_err(ProcOper::Parsing, ProcFile::PidStat, e))
            )
        );
        Self::parse_string(bytes)
    }

    /// Parse a String as a /proc/[pid]/stat file.
    fn parse_string(bytes: String) -> Result<Self, ProcError> {
        // /proc/.../stat is "numbers (prog_name) char numbers"
        // prog_name could have arbitrary characters, so we need to parse
        // the file from both ends
        let mut bytes_split = bytes.splitn(2, '(');
        let prefix = try!(bytes_split.next()
            .ok_or(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("finding opening paren"))));
        let mut bytes_split = match bytes_split.next() {
            Some(b) => b.rsplitn(2, ')'),
            None => return Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat,
                                                 Some("finding closing paren")))
        };
        // /proc/.../stat has a newline at the end
        let suffix = try!(bytes_split.next()
            .ok_or(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("splitting file")))
            ).trim();
        let prog_name = try!(bytes_split.next()
            .ok_or(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("splitting comm"))));
        let mut split = suffix.split(' ');

        Ok(PidStat {
            pid: stat_parse_num!(prefix.split(' ').next()),
            // From here parse from back, since arbitrary data can be in program name
            comm: prog_name.to_owned(),
            state: try!(
                split.next()
                    .and_then(|s|
                        get_procstate(s)
                    ).ok_or(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat,
                                                                            Some("parsing process state")))
            ),
            ppid: stat_parse_num!(split.next()),
            pgrp: stat_parse_num!(split.next()),
            session: stat_parse_num!(split.next()),
            tty_nr: stat_parse_num!(split.next()),
            tpgid: stat_parse_num!(split.next()),
            flags: stat_parse_num!(split.next()),
            minflt: stat_parse_num!(split.next()),
            cminflt: stat_parse_num!(split.next()),
            majflt: stat_parse_num!(split.next()),
            cmajflt: stat_parse_num!(split.next()),
            utime: stat_parse_num!(split.next()),
            stime: stat_parse_num!(split.next()),
            cutime: stat_parse_num!(split.next()),
            cstime: stat_parse_num!(split.next()),
            priority: stat_parse_num!(split.next()),
            nice: stat_parse_num!(split.next()),
            num_threads: stat_parse_num!(split.next()),
            itrealvalue: stat_parse_num!(split.next()),
            starttime: stat_parse_num!(split.next()),
            vsize: stat_parse_num!(split.next()),
            rss: stat_parse_num!(split.next()),
            rsslim: stat_parse_num!(split.next()),
            startcode: stat_parse_num!(split.next()),
            endcode: stat_parse_num!(split.next()),
            startstack: stat_parse_num!(split.next()),
            kstkesp: stat_parse_num!(split.next()),
            kstkeip: stat_parse_num!(split.next()),
            signal: stat_parse_num!(split.next()),
            blocked: stat_parse_num!(split.next()),
            sigignore: stat_parse_num!(split.next()),
            sigcatch: stat_parse_num!(split.next()),
            wchan: stat_parse_num!(split.next()),
            nswap: stat_parse_num!(split.next()),
            cnswap: stat_parse_num!(split.next()),
            exit_signal:
                stat_parse_opt_num!(split.next()),
            processor:
                stat_parse_opt_num!(split.next()),
            rt_priority:
                stat_parse_opt_num!(split.next()),
            policy:
                stat_parse_opt_num!(split.next()),
            delayacct_blkio_ticks:
                stat_parse_opt_num!(split.next()),
            guest_time:
                stat_parse_opt_num!(split.next()),
            cguest_time:
                stat_parse_opt_num!(split.next()),
            start_data:
                stat_parse_opt_num!(split.next()),
            end_data:
                stat_parse_opt_num!(split.next()),
            start_brk:
                stat_parse_opt_num!(split.next()),
            arg_start:
                stat_parse_opt_num!(split.next()),
            arg_end:
                stat_parse_opt_num!(split.next()),
            env_start:
                stat_parse_opt_num!(split.next()),
            env_end:
                stat_parse_opt_num!(split.next()),
            exit_code:
                stat_parse_opt_num!(split.next()),
        })
    }
}

/// A list of states that a process can be in.
#[derive(Debug, Clone, PartialEq)]
pub enum PidState {
    /// Running
    Running,
    /// Sleeping in an interruptible wait
    Sleeping,
    /// Waiting in an uninterruptible disk sleep
    Waiting,
    /// Zombie
    Zombie,
    /// Stopped (on a signal) or (before LInux 2.6.33) trace stopped
    Stopped,
    /// Tracing stop
    Tracing,
    /// Dead
    Dead,
    /// Wakekill
    Wakekill,
    /// Waking
    Waking,
    /// Parked
    Parked
}

/// Turn a char into an appropriate ProcState.
fn get_procstate(state: &str) -> Option<PidState> {
    match state {
        "R" => Some(PidState::Running),
        "S" => Some(PidState::Sleeping),
        "D" => Some(PidState::Waiting),
        "Z" => Some(PidState::Zombie),
        "T" => Some(PidState::Stopped),
        "t" => Some(PidState::Tracing),
        "X" | "x" => Some(PidState::Dead),
        "K" => Some(PidState::Wakekill),
        "W" => Some(PidState::Waking),
        "P" => Some(PidState::Parked),
         _  => None
    }
}

#[test]
fn test_parsing() {
    let test_prc = PidStat{
        pid: 14557,
        comm: "psq".to_owned(),
        state: PidState::Stopped,
        ppid: 14364,
        pgrp: 14557,
        session: 14364,
        tty_nr: 34823,
        tpgid: 14638,
        flags: 1077952512,
        minflt: 1178,
        cminflt: 0,
        majflt: 0,
        cmajflt: 0,
        utime: 16,
        stime: 0,
        cutime: 0,
        cstime: 0,
        priority: 20,
        nice: 0,
        num_threads: 1,
        itrealvalue: 0,
        starttime: 609164,
        vsize: 23785472,
        rss: 1707,
        rsslim: 18446744073709551615,
        startcode: 94178658361344,
        endcode: 94178659818816,
        startstack: 140735096462144,
        kstkesp: 140735096450384,
        kstkeip: 94178659203252,
        signal: 0,
        blocked: 0,
        sigignore: 4224,
        sigcatch: 1088,
        wchan: 1,
        nswap: 0,
        cnswap: 0,
        exit_signal: Some(17),
        processor: Some(2),
        rt_priority: Some(0),
        policy: Some(0),
        delayacct_blkio_ticks: Some(0),
        guest_time: Some(0),
        cguest_time: Some(0),
        start_data: Some(94178661916280),
        end_data: Some(94178661971297),
        start_brk: Some(94178690334720),
        arg_start: Some(140735096465030),
        arg_end: Some(140735096465049),
        env_start: Some(140735096465049),
        env_end: Some(140735096467429),
        exit_code: Some(0)
    };

    let input = "14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input), Ok(test_prc));
}

// For each of the following tests, the previous text input is used to create a PidStat struct.

#[test]
fn test_state_running() {
    let mut prc = PidStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
    prc.state = PidState::Running;
    let input = "14557 (psq) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input), Ok(prc));
}

#[test]
fn test_comm_space() {
    let mut prc = PidStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
    prc.state = PidState::Running;
    prc.comm = "psq ".to_owned();
    let input = "14557 (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input), Ok(prc));
}

#[test]
fn test_double_space() {
    let mut prc = PidStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
    prc.state = PidState::Running;
    prc.comm = "psq ".to_owned();
    let input = "14557  (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input), Ok(prc));
}

#[test]
fn test_comm_parens() {
    let mut prc = PidStat::parse_string("14557 (psq) T 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned()).unwrap();
    prc.state = PidState::Running;
    prc.comm = " ) (psq ".to_owned();
    let input = "14557  ( ) (psq ) R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input), Ok(prc));
}

#[test]
fn test_invalid_parens() {
    let input = "14557   ) (psq (R 14364 14557 14364 34823 14638 1077952512 1178 0 0 0 16 0 0 0 20 0 1 0 609164 23785472 1707 18446744073709551615 94178658361344 94178659818816 140735096462144 140735096450384 94178659203252 0 0 4224 1088 1 0 0 17 2 0 0 0 0 0 94178661916280 94178661971297 94178690334720 140735096465030 140735096465049 140735096465049 140735096467429 0".to_owned();
    assert_eq!(PidStat::parse_string(input),
        Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("splitting comm"))));
}

#[test]
fn test_invalid_1() {
    let input = "14557 ".to_owned();
    assert_eq!(PidStat::parse_string(input),
        Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("finding closing paren"))));
}

#[test]
fn test_invalid_2() {
    let input = "14557 (a) 3".to_owned();
    assert_eq!(PidStat::parse_string(input),
        Err(ProcError::new_more(ProcOper::Parsing, ProcFile::PidStat, Some("parsing process state"))));
}
