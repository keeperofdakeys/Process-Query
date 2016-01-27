use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

pub type ProcPid = u32;

#[derive(Debug)]
pub struct Proc {
  pid: ProcPid,
  ppid: Option<ProcPid>,
  tgid: Option<ProcPid>,
  name: Option<String>,
  cmdline: Option<String>
}

impl Proc {
  pub fn new(pid: ProcPid) -> Result<Proc, String> {
    let mut proc_q = Proc{
      pid: pid,
      ppid: None,
      tgid: None,
      name: None,
      cmdline: None,
    };
    let proc_dir = format!("/proc/{}", pid);
    try!(
      proc_q
        .read_status(&proc_dir)
        .and_then(|proc_q| proc_q.read_cmdline(&proc_dir))
    );
    println!("{:?}", proc_q);
    Ok(proc_q)
  }

  fn read_status(&mut self, proc_dir: &str) -> Result<&mut Self, String> {

    let status_file = try!(
      File::open(Path::new(proc_dir).join("status"))
        .map_err(|err| err.to_string())
    );
    for line in BufReader::new(status_file).lines() {
      try!(
        line
          .map_err(|err| err.to_string())
          .and_then(|line| {
            let split = line.splitn(2, ':').collect::<Vec<&str>>();

            let key = split.get(0).map(|k| k.trim());
            let value = split.get(1).map(|v| v.trim());

            let (key, value) = match (split.get(0), split.get(1)) {
              (Some(k), Some(v)) => (k.trim(), v.trim()),
              _ => return Err("Error reading line".to_string())
            };

            match key {
              "PPid" => self.ppid = value.parse().ok(),
              "Tgid" => self.tgid = value.parse().ok(),
              "Name" => self.name = Some(value.to_string()),
              _ => {}
              //_ => return Err(format!("Unknown status key '{}'", key))
            };
            Ok(())
          })
        );
    }
    Ok(self)
  }

  fn read_cmdline(&mut self, proc_dir: &str) -> Result<&mut Self, String> {
    self.cmdline = Some(
      try!(
        File::open(Path::new(proc_dir).join("cmdline"))
          .map_err(|err| err.to_string())
          .and_then(|mut file| {
            let mut contents = Vec::new();
            try!(
              file.read_to_end(&mut contents)
                .map_err(|err| err.to_string())
            );
            String::from_utf8(contents)
              .map_err(|err| err.to_string())
          })
        )
    );
    Ok(self)
  }
}

impl PartialEq for Proc {
  fn eq(&self, other: &Proc) -> bool {
    return self.pid == other.pid;
  }
}
