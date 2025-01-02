mod bitmap;
mod common;
mod ansi;

use std::io::Error;
use std::env;

use bitmap::bitmap::Bitmap;

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Should not fail");
    let term_height = term_size.rows as usize;
    let term_width = term_size.cols as usize;
    println!("height: {term_height}, width: {term_width}");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [filename]"));
    }
    
    let bitmap = Bitmap::new(&args[1])?;
    bitmap.print(term_height, term_width);

    Ok(())
}
