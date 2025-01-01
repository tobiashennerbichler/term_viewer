mod bitmap;

use std::io::Error;
use std::env;

use bitmap::bitmap::{Bitmap, parse_bitmap_header};

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Should not fail");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [filename]"));
    }
    
    parse_bitmap_header(b"BM6\xEB\x41\x00\x00\x00\x00\x006\x00\x00\x00\x28\x00\x00\x00\xB0\x04")?;
    
    Ok(())
}
