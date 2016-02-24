use std::fmt;
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
    match *self {
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
pub struct ProcError {
  error: ProcOper,
  file: ProcFile,
  inner: Option<Box<Error>>,
  more: Option<&'static str>
}

impl ProcError {
  pub fn new_err<E: Error + 'static>(error: ProcOper, file: ProcFile, cause: E)
    -> ProcError {
    ProcError {
      error: error,
      file: file,
      inner: Some(Box::new(cause)),
      more: None
    }
  }

  pub fn new_more(error: ProcOper, file: ProcFile, more: Option<&'static str>) -> ProcError {
    ProcError {
      error: error,
      file: file,
      inner: None,
      more: more
    }
  }

  pub fn new<E: Error + 'static>(error: ProcOper, file: ProcFile, cause: Option<E>,
    more: Option<&'static str>) -> ProcError {
    ProcError {
      error: error,
      file: file,
      inner: match cause {
        Some(e) => Some(Box::new(e)),
        None => None
      },
      more: more
    }
  }

  pub fn is_hard(&self) -> bool {
    self.error.is_hard()
  }
}

impl Error for ProcError {
  fn description(&self) -> &str {
    self.error.description()
  }

  fn cause(&self) -> Option<&'static Error> {
    None
    // self.inner
  }
}

impl fmt::Debug for ProcError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let more = self.more.unwrap_or("");
    let error;
    if let Some(e) = self.inner.as_ref() {
      error = e.description();
    } else {
      error = "";
    }
    write!(f, "error {} ({}) from {}: {}",
      self.error.description(), more,
      self.file.description(), error)
  }
}

impl fmt::Display for ProcError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

impl PartialEq for ProcError {
  fn eq(&self, other: &Self) -> bool {
    self.error.eq(&other.error) && self.file.eq(&other.file) && self.more.eq(&other.more)
  }
}
