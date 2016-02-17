use std::fmt;

// Fields in a Proc
#[derive(Clone, Debug, PartialEq)]
pub enum ProcPart {
  ProcPartStat,
  ProcPartStatus,
  ProcPartCmdline
}

impl fmt::Display for ProcPart {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}",
      match *self {
        ProcPartStat => "stat",
        ProcPartStatus => "status",
        ProcPartCmdline => "cmdline"
      }
    )
  }
}

// Error types that can occur making a Proc
#[derive(Clone, Debug, PartialEq)]
pub enum ProcErrorType {
  ProcParseError,
  ProcReadError,
}

impl fmt::Display for ProcErrorType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}",
      match *self {
        ProcParseError => "parsing the file",
        ProcReadError => "reading the file"
      }
    )
  }
}

// An error that occurs during parsing
#[derive(Clone, Debug, PartialEq)]
pub enum ProcError {
  // A soft error is something that is temporary, or recoverable.
  // For example, trying to read a /proc file for an invalid pid.
  ProcSoftError(ProcErrorType, ProcPart),
  // A hard error is something that is unrecoverable.
  // For example, a missing /proc, or a parsing error.
  ProcHardError(ProcErrorType, ProcPart),
}

impl ProcError {
  pub fn is_hard(&self) -> bool {
    return match *self {
      ProcHardError(..) => true,
      ProcSoftError(..) => false
    }
  }
}

impl fmt::Display for ProcError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let (e_type, part);
    let diag = match *self {
      ProcError::ProcHardError(ref t, ref p) => {
        e_type = t;
        part = p;
        "hard"
      },
      ProcError::ProcSoftError(ref t, ref p) => {
        e_type = t;
        part = p;
        "soft"
      }
    };
    write!(f, "A '{}' error occured while '{}' the '{}' part.",
      diag, e_type, part)
  }
}

// Export enum variants
pub use self::ProcError::*;
pub use self::ProcErrorType::*;
pub use self::ProcPart::*;
