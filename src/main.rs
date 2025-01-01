mod bitmap;
mod common;

use std::io::Error;
use std::env;

use bitmap::bitmap::Bitmap;

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Should not fail");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [filename]"));
    }
    
    let bitmap = Bitmap::new(&args[1])?;
    for y in 0..bitmap.height {
        for x in 0..bitmap.width {
            bitmap.pixels[y][x].print();
        }
        println!("");
    }
    
    Ok(())
}
