extern crate procrs;
#[macro_use]
extern crate prettytable;

use procrs::meminfo;
use prettytable::Table;
use prettytable::format::FormatBuilder;
use prettytable::format::Alignment;


fn main () {
    // Build the minfo
    let minfo = match meminfo::Meminfo::new() {
        Ok(minfo) => minfo,
        Err(err) => { println!("ERROR, {:?}", err); return },
    };
    // println!("{:?}", minfo);
    // Make it look like this :) 
    //               total        used        free      shared  buff/cache   available
    // Mem:       12202716     1666600      957368      401652     9578748     9989056
    // Swap:       6160380           0     6160380

    // Start building the table
    let mut table = Table::new();
    // Need to calculate used from other things
    table.add_row(row!["", "total", "used", "free", "shared", "buff/cache", "available"]);
    table.add_row(row!["Mem:", minfo.memtotal, minfo.mainused, minfo.memfree, minfo.shmem, minfo.maincached, minfo.memavailable]);
    table.add_row(row!["Swap:", minfo.swaptotal, minfo.mainswapused, minfo.swapfree]);
    // Make a format for it
    let format = FormatBuilder::new()
        .column_separator(' ')
        .padding(0, 3)
        .build();
    table.set_format(format);
    
    for r in table.row_iter_mut() {
        for cel in r.iter_mut() {
            cel.align(Alignment::RIGHT);
        }
    }

    for cel in table.column_iter_mut(0) {
        cel.align(Alignment::LEFT);
    }

    table.printstd();


}

