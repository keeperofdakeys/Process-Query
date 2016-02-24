use std::fmt;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::collections::HashMap;

#[derive(Debug)]
pub enum MeminfoError {
    Io(io::Error),
    NotFound,
}

impl fmt::Display for MeminfoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MeminfoError::Io(ref err) => err.fmt(f),
            MeminfoError::NotFound => write!(f, "Unknown error occured"),
        }
    }
}

impl From<io::Error> for MeminfoError {
    fn from(err: io::Error) -> MeminfoError {
        MeminfoError::Io(err)
    }
}

#[derive(Debug)]
pub struct MeminfoStatus {
// MemTotal:       12202880 kB
    memtotal: u64,
// MemFree:         1927408 kB
    memfree: u64,
// MemAvailable:    8288884 kB
    memavailable: u64,
// Buffers:               8 kB
// Cached:          6253764 kB
// SwapCached:       254028 kB
// Active:          5520944 kB
// Inactive:        3843272 kB
// Active(anon):    2010724 kB
// Inactive(anon):  1442068 kB
// Active(file):    3510220 kB
// Inactive(file):  2401204 kB
// Unevictable:       22312 kB
// Mlocked:           22312 kB
// SwapTotal:       6160380 kB
// SwapFree:        4597788 kB
// Dirty:               392 kB
// Writeback:             0 kB
// AnonPages:       2960724 kB
// Mapped:           397256 kB
// Shmem:            333604 kB
// Slab:             644044 kB
// SReclaimable:     502180 kB
// SUnreclaim:       141864 kB
// KernelStack:       13216 kB
// PageTables:        56408 kB
// NFS_Unstable:          0 kB
// Bounce:                0 kB
// WritebackTmp:          0 kB
// CommitLimit:    12261820 kB
// Committed_AS:   10955940 kB
// VmallocTotal:   34359738367 kB
// VmallocUsed:      685436 kB
// VmallocChunk:   34358161404 kB
// HardwareCorrupted:     0 kB
// AnonHugePages:         0 kB
// HugePages_Total:       0
// HugePages_Free:        0
// HugePages_Rsvd:        0
// HugePages_Surp:        0
// Hugepagesize:       2048 kB
// DirectMap4k:      738332 kB
// DirectMap2M:    11743232 kB
// DirectMap1G:     1048576 kB
}


/// Parses the contents of /proc/meminfo into a new Meminfo structure
///
/// # Examples

impl MeminfoStatus {
    pub fn new() -> Result<Self, MeminfoError> {
        // Create an interim hashmap
        // Read the file?
        let minfo_file: File = try!(File::open("/proc/meminfo"));
        // Parse the file
        // How to we make sure this error is propogated correctly?
        let lines = try!(io::BufReader::new(minfo_file)
            .lines() // We have a Lines of many Result<&str>
            .collect::<Result<Vec<_>, _>>()); // This line makes Result<vec<&str>> Or result<err>
        let hmap = try!(lines.iter().map(|line| Self::parse_line(line)).collect::<Result<HashMap<_, _>, _>>()  );
        // Populate the results
        Self::build_minfo(hmap)
    }

    // This builds up the hash map.
    fn parse_line(line: &str) -> Result<(String, u64), MeminfoError> {
        // Find the : offset
        let mut lineiter = line.split_whitespace();
        let key = lineiter.next().unwrap().trim_matches(':');
        let value = lineiter.next().unwrap().parse::<u64>().unwrap();
        // trim and parse to int
        Ok((key.to_owned(), value))
    }

    //This then takes the values out and puts them into an minfo
    fn build_minfo(hmap: HashMap<String, u64>) -> Result<MeminfoStatus, MeminfoError> {
        println!("{:?}", hmap);
        // REALLY REALLY improve this handling of Option types ...
        let minfo = MeminfoStatus {
            memtotal: hmap.get("MemTotal").unwrap().clone(),
            memfree: hmap.get("MemFree").unwrap().clone(),
            memavailable: hmap.get("MemAvailable").unwrap().clone(),
        };
        Ok(minfo)
    }

}


impl fmt::Display for MeminfoStatus {
    // make a display method to dump the whole struct
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This won't be nice for all the values we have ...
        write!(f, "Memtotal: {}\nMemFree: {}", self.memtotal, self.memfree )
    }
}

    // make a pretty print for the format of free
    // Should it accept display units?


