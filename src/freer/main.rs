extern crate procrs;
use procrs::meminfo;

fn main () {
    match meminfo::MeminfoStatus::new() {
        Ok(minfo) => println!("{}", minfo),
        Err(err) => println!("ERROR, {}", err),
    }
}

