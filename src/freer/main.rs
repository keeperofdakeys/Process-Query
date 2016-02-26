extern crate procrs;
#[macro_use]
extern crate prettytable;

use procrs::meminfo;
use prettytable::Table;
use prettytable::format::FormatBuilder;


fn main () {
    // Build the minfo
    let minfo = match meminfo::MeminfoStatus::new() {
        Ok(minfo) => minfo,
        Err(err) => { println!("ERROR, {:?}", err); return },
    };
    // make a debug option that prints this nicely.
    //println!("{:?}", minfo);
    // Make it look like this :) 
    //               total        used        free      shared  buff/cache   available
    // Mem:       12202716     1666600      957368      401652     9578748     9989056
    // Swap:       6160380           0     6160380

    // Start building the table
    let mut table = Table::new();
    // Need to calculate used from other things
    table.add_row(row!["Mem:", minfo.memtotal, 0, minfo.memfree]);
    // Make a format for it
    let format = FormatBuilder::new()
        .column_separator(' ')
        .padding(0, 12)
        .build();
    table.set_format(format);
    table.set_titles(row!["", "total", "used", "free"]);
    table.printstd();

    // Now push data into it and display
    

}

