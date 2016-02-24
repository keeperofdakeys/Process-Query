use std::fmt;
use std::io;
use std::io::Write;

#[derive(PartialEq)]
// An enum representing files in a /proc/pid/ dir
pub enum PidFile {
  PidDir,
  PidStat,
  PidStatus,
  PidCmdline
}

impl fmt::Debug for PidFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      PidFile::PidDir => write!(f, "Process Directory"),
      PidFile::PidStat => write!(f, "Process Stat File"),
      PidFile::PidStatus => write!(f, "Process Status File"),
      PidFile::PidCmdline => write!(f, "Process Cmdline File")
    }
  }
}

impl fmt::Display for PidFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

// An error that occurs while creating a process Prc
pub enum PidError {
  // Error opening a file/dir
  Opening(PidFile, io::Error),
  // Error reading a file/dir
  Reading(PidFile, io::Error),
  // 
  Parsing(PidFile, &'static str),
  Field(PidFile, &'static str)
}

impl PidError {
  pub fn is_hard(&self) -> bool {
    return match *self {
      PidError::Opening(_, _) => false,
      PidError::Reading(_, _) => false,
      _ => true
    }
  }
}

impl fmt::Debug for PidError {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      PidError::Opening(ref f, ref e) => write!(fmt, "An error occured opening a {}: {}", f, e),
      PidError::Reading(ref f, ref e) => write!(fmt, "An error occured reading a {}: {}", f, e),
      PidError::Parsing(ref f, ref e) => write!(fmt, "An error occured parsing a {}: {}", f, e),
      PidError::Field(ref f, ref e) => write!(fmt, "An error occured parsing a field in a {}: {}", f, e)
    }
  }
}

impl fmt::Display for PidError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

impl PartialEq for PidError {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (&PidError::Opening(ref f1, _), &PidError::Opening(ref f2, _))
        if f1 == f2 => true,
      (&PidError::Reading(ref f1, _), &PidError::Reading(ref f2, _))
        if f1 == f2 => true,
      (&PidError::Parsing(ref f1, ref e1), &PidError::Parsing(ref f2, ref e2))
        if f1 == f2 && e1 == e2 => true,
      (&PidError::Field(ref f1, ref e1), &PidError::Field(ref f2, ref e2))
        if f1 == f2 && e1 == e2 => true,
      _ => false
    }
  }
}
