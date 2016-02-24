use std::fmt;
use std::io;
use std::io::Write;
use std::error::Error;

/// A list of files contained in the /proc directory>
///
/// This list is used to identify which file or directory an error is relating too.
#[derive(PartialEq)]
pub enum ProcFile {
  /// /proc Directory, contains files containg various pieces of information about the system.
  ProcDir,
  /// /proc/cmdline file, contains the cmdline used when starting the kernel.
  ProcCmdline,
  /// /proc/cpuinfo file, contains information about the cpu.
  ProcCpuinfo,
  /// /proc/meminfo file, contains information about the memory resources of the system.
  ProcMeminfo,
  /// /proc/stat file.
  ProcStat,
  /// /proc/uptime file, contains the uptime of the system.
  ProcUptime,
  /// /proc/status file.
  ProcStatus,

  /// /proc/[pid] directory, contains files relating to the process at [pid].
  PidDir,
  /// /proc/[pid]/status file, contains various human-readable stats about the process.
  PidStatus,
  /// /proc/[pid]/stat file, contains various stats about the process.
  PidStat,
  /// /proc/[pid]/cmdline file, contains the cmdline given when starting the process.
  PidCmdline
}

impl Error for ProcFile {
  fn description(&self) -> &str {
    match *self {
      ProcFile::ProcDir => "/proc directory",
      ProcFile::ProcCmdline => "/proc/cmdline file",
      ProcFile::ProcCpuinfo => "/proc/cmdinfo file",
      ProcFile::ProcMeminfo => "/proc/meminfo file",
      ProcFile::ProcStat => "/proc/stat file",
      ProcFile::ProcUptime => "/proc/uptime file",
      ProcFile::ProcStatus => "/proc/status file",
      ProcFile::PidDir => "/proc/[pid] directory",
      ProcFile::PidStatus => "/proc/[pid]/status file",
      ProcFile::PidStat => "/proc/[pid]/stat file",
      ProcFile::PidCmdline => "/proc/[pid]/cmdline file"
    }
  }

  fn cause(&self) -> Option<&Error> {
    None
  }
}

impl fmt::Debug for ProcFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.description())
  }
}

impl fmt::Display for ProcFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.description())
  }
}

/// A list of errors that can occur while operating on something in /proc.
#[derive(PartialEq)]
pub enum ProcOper {
  /// Error opening a file/directory.
  Opening,
  /// Error reading a file/directorr.
  Reading,
  /// Error parsing a file/directory.
  Parsing,
  /// Error parsing a specific field in a file/directory.
  ParsingField,
}

impl ProcOper {
  pub fn is_hard(&self) -> bool {
    return match *self {
      ProcOper::Opening => false,
      ProcOper::Reading => false,
      _ => true
    }
  }
}

impl Error for ProcOper {
  fn description(&self) -> &'static str {
    match *self {
      ProcOper::Opening => "opening",
      ProcOper::Reading => "reading",
      ProcOper::Parsing => "parsing",
      ProcOper::ParsingField => "parsing field"
    }
  }
}

impl fmt::Debug for ProcOper {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.description())
  }
}

impl fmt::Display for ProcOper {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.description())
  }
}

// The error type for operations on /proc.
//
// Errors that can occur while reading /proc. These have an error
// kind (error), a file/directory (file), an inner error (inner)
// and optionally more information that is error-specific.
pub struct ProcError<'a> {
  error: ProcOper,
  file: ProcFile,
  inner: Option<&'a Error>,
  more: Option<&'a str>
}

impl<'a> ProcError<'a> {
  pub fn new(error: ProcOper, file: ProcFile) -> ProcError<'a> {
    ProcError {
      error: error,
      file: file,
      inner: None,
      more: None
    }
  }

  pub fn new_err(error: ProcOper, file: ProcFile, cause: &'a Error)
    -> ProcError<'a> {
    ProcError {
      error: error,
      file: file,
      inner: Some(cause),
      more: None
    }
  }

  pub fn new_more(error: ProcOper, file: ProcFile, cause: &'a Error,
    more: &'a str) -> ProcError<'a> {
    ProcError {
      error: error,
      file: file,
      inner: Some(cause),
      more: Some(more)
    }
  }
}

impl<'a> Error for ProcError<'a> {
  fn description(&self) -> &str {
    self.error.description()
  }

  fn cause(&self) -> Option<&Error> {
    self.inner
  }
}

impl<'a> fmt::Debug for ProcError<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.error {
      ProcOper::ParsingField if self.more.is_some() => match self.inner {
        Some(e) =>
          write!(f, "error {} {} from {}: {}", self.error.description(),
            self.more.unwrap(), self.file.description(), e),
        None =>
          write!(f, "error {} {} from {}", self.error.description(),
            self.more.unwrap(), self.file.description())
      },
      _ => match self.more {
        Some(e) =>
          write!(f, "error {} from {}: {}", self.error.description(),
            self.file.description(), e),
        None =>
          write!(f, "error {} from {}", self.error.description(),
            self.file.description())
      }
    }
  }
}

impl<'a> fmt::Display for ProcError<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

impl<'a> PartialEq for ProcError<'a> {
  fn eq(&self, other: &Self) -> bool {
    self.error.eq(&other.error) && self.file.eq(&other.file)
  }
}
