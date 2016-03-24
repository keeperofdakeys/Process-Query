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
pub struct Meminfo {
    pub memtotal: u64,
    pub memfree: u64,
    pub memavailable: u64,
    pub buffers: u64,
    pub cached: u64,
    pub swapcached: u64,
    pub active: u64,
    pub inactive: u64,
    pub activeanon: u64,
    pub inactiveanon: u64,
    pub activefile: u64,
    pub inactivefile: u64,
    pub unevictable: u64,
    pub mlocked: u64,
    pub swaptotal: u64,
    pub swapfree: u64,
    pub dirty: u64,
    pub writeback: u64,
    pub anonpages: u64,
    pub mapped: u64,
    pub shmem: u64,
    pub slab: u64,
    pub srelclaimable: u64,
    pub sunreclaim: u64,
    pub kernelstack: u64,
    pub pagetables: u64,
    pub nfsunstable: u64,
    pub bounce: u64,
    pub writebacktmp: u64,
    pub commitlimit: u64,
    pub committedas: u64,
    pub vmalloctotal: u64,
    pub vmallocused: u64,
    pub vmallocchunk: u64,
    pub hardwarecorrupted: u64,
    pub anonhugepages: u64,
    pub hugepagestotal: u64,
    pub hugepagesfree: u64,
    pub hugepagsersvd: u64,
    pub hugepagessurp: u64,
    pub hugepagessize: u64,
    pub directmap4k: u64,
    pub directmap2m: u64,
    // pub directmap1g: u64,
    pub mainused: u64,
    pub maincached: u64,
    pub mainswapused: u64,
}


/// Parses the contents of /proc/meminfo into a new Meminfo structure
///
/// # Examples

impl Meminfo {
    pub fn new() -> Result<Self, MeminfoError> {
        // Create an interim hashmap
        // Read the file?
        let minfo_file: File = try!(File::open("/proc/meminfo"));
        // Parse the file
        // How to we make sure this error is propogated correctly?
        let lines = try!(io::BufReader::new(minfo_file)
            .lines() // We have a Lines of many Result<&str>
            .collect::<Result<Vec<_>, _>>()); // This line makes Result<vec<&str>> Or result<err>
        let mut hmap = try!(lines.iter().map(|line| Self::parse_line(line)).collect::<Result<HashMap<_, _>, _>>()  );
        //  Calculate some of the other values
        // kb_main_used = kb_main_total - kb_main_free - kb_main_cached - kb_main_buffe
        let total = hmap.get("MemTotal").unwrap().clone();
        let free = hmap.get("MemFree").unwrap().clone();
        let cached = hmap.get("Cached").unwrap().clone();
        let buffer = hmap.get("Buffers").unwrap().clone();
        let used = total - free - cached - buffer;
        hmap.insert("MainUsed".to_owned(), used);

        // kb_main_cached = kb_page_cache + kb_slab
        let page_cache = hmap.get("Cached").unwrap().clone();
        let slab = hmap.get("Slab").unwrap().clone();
        hmap.insert("MainCached".to_owned(), (page_cache + slab) );

        // kb_swap_used = kb_swap_total - kb_swap_free
        let swap_total = hmap.get("SwapTotal").unwrap().clone();
        let swap_free = hmap.get("SwapFree").unwrap().clone();
        hmap.insert("MainSwapUsed".to_owned(), (swap_total - swap_free));

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
    fn build_minfo(hmap: HashMap<String, u64>) -> Result<Meminfo, MeminfoError> {
        // REALLY REALLY improve this handling of Option types ...
        let minfo = Meminfo {
            memtotal: hmap.get("MemTotal").unwrap().clone(),
            memfree: hmap.get("MemFree").unwrap().clone(),
            memavailable: hmap.get("MemAvailable").unwrap().clone(),
            buffers: hmap.get("Buffers").unwrap().clone(),
            cached: hmap.get("Cached").unwrap().clone(),
            swapcached: hmap.get("SwapCached").unwrap().clone(),
            active: hmap.get("Active").unwrap().clone(),
            inactive: hmap.get("Inactive").unwrap().clone(),
            activeanon: hmap.get("Active(anon)").unwrap().clone(),
            inactiveanon: hmap.get("Inactive(anon)").unwrap().clone(),
            activefile: hmap.get("Active(file)").unwrap().clone(),
            inactivefile: hmap.get("Inactive(file)").unwrap().clone(),
            unevictable: hmap.get("Unevictable").unwrap().clone(),
            mlocked: hmap.get("Mlocked").unwrap().clone(),
            swaptotal: hmap.get("SwapTotal").unwrap().clone(),
            swapfree: hmap.get("SwapFree").unwrap().clone(),
            dirty: hmap.get("Dirty").unwrap().clone(),
            writeback: hmap.get("Writeback").unwrap().clone(),
            anonpages: hmap.get("AnonPages").unwrap().clone(),
            mapped: hmap.get("Mapped").unwrap().clone(),
            shmem: hmap.get("Shmem").unwrap().clone(),
            slab: hmap.get("Slab").unwrap().clone(),
            srelclaimable: hmap.get("SReclaimable").unwrap().clone(),
            sunreclaim: hmap.get("SUnreclaim").unwrap().clone(),
            kernelstack: hmap.get("KernelStack").unwrap().clone(),
            pagetables: hmap.get("PageTables").unwrap().clone(),
            nfsunstable: hmap.get("NFS_Unstable").unwrap().clone(),
            bounce: hmap.get("Bounce").unwrap().clone(),
            writebacktmp: hmap.get("WritebackTmp").unwrap().clone(),
            commitlimit: hmap.get("CommitLimit").unwrap().clone(),
            committedas: hmap.get("Committed_AS").unwrap().clone(),
            vmalloctotal: hmap.get("VmallocTotal").unwrap().clone(),
            vmallocused: hmap.get("VmallocUsed").unwrap().clone(),
            vmallocchunk: hmap.get("VmallocChunk").unwrap().clone(),
            hardwarecorrupted: hmap.get("HardwareCorrupted").unwrap().clone(),
            anonhugepages: hmap.get("AnonHugePages").unwrap().clone(),
            hugepagestotal: hmap.get("HugePages_Total").unwrap().clone(),
            hugepagesfree: hmap.get("HugePages_Free").unwrap().clone(),
            hugepagsersvd: hmap.get("HugePages_Rsvd").unwrap().clone(),
            hugepagessurp: hmap.get("HugePages_Surp").unwrap().clone(),
            hugepagessize: hmap.get("Hugepagesize").unwrap().clone(),
            directmap4k: hmap.get("DirectMap4k").unwrap().clone(),
            directmap2m: hmap.get("DirectMap2M").unwrap().clone(),
            // directmap1g: hmap.get("DirectMap1G").unwrap().clone(),
            mainused: hmap.get("MainUsed").unwrap().clone(),
            maincached: hmap.get("MainCached").unwrap().clone(),
            mainswapused: hmap.get("MainSwapUsed").unwrap().clone(),
        };
        Ok(minfo)
    }

}


impl fmt::Display for Meminfo {
    // make a display method to dump the whole struct
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This won't be nice for all the values we have ...
        write!(f, "{:?}", self )
    }
}

    // make a pretty print for the format of free
    // Should it accept display units?


