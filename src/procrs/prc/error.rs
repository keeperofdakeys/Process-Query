use std::fmt;
use std::io;
use std::io::Write;

#[derive(PartialEq)]
// An enum representing files in a /proc/pid/ dir
pub enum PrcFile {
  PrcDir,
  PrcStat,
  PrcStatus,
  PrcCmdline
}

impl fmt::Debug for PrcFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      PrcFile::PrcDir => write!(f, "Process Directory"),
      PrcFile::PrcStat => write!(f, "Process Stat File"),
      PrcFile::PrcStatus => write!(f, "Process Status File"),
      PrcFile::PrcCmdline => write!(f, "Process Cmdline File")
    }
  }
}

impl fmt::Display for PrcFile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

// An error that occurs while creating a process Prc
pub enum PrcError {
  // Error opening a file/dir
  Opening(PrcFile, io::Error),
  // Error reading a file/dir
  Reading(PrcFile, io::Error),
  // 
  Parsing(PrcFile, &'static str),
  Field(PrcFile, &'static str)
}

impl PrcError {
  pub fn is_hard(&self) -> bool {
    return match *self {
      PrcError::Opening(_, _) => false,
      PrcError::Reading(_, _) => false,
      _ => true
    }
  }
}

impl fmt::Debug for PrcError {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      PrcError::Opening(ref f, ref e) => write!(fmt, "An error occured opening a {}: {}", f, e),
      PrcError::Reading(ref f, ref e) => write!(fmt, "An error occured reading a {}: {}", f, e),
      PrcError::Parsing(ref f, ref e) => write!(fmt, "An error occured parsing a {}: {}", f, e),
      PrcError::Field(ref f, ref e) => write!(fmt, "An error occured parsing a field in a {}: {}", f, e)
    }
  }
}

impl fmt::Display for PrcError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

impl PartialEq for PrcError {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (&PrcError::Opening(ref f1, _), &PrcError::Opening(ref f2, _))
        if f1 == f2 => true,
      (&PrcError::Reading(ref f1, _), &PrcError::Reading(ref f2, _))
        if f1 == f2 => true,
      (&PrcError::Parsing(ref f1, ref e1), &PrcError::Parsing(ref f2, ref e2))
        if f1 == f2 && e1 == e2 => true,
      (&PrcError::Field(ref f1, ref e1), &PrcError::Field(ref f2, ref e2))
        if f1 == f2 && e1 == e2 => true,
      _ => false
    }
  }
}
