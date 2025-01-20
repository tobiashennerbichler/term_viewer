mod bitmap;
mod common;
mod ansi;
mod handler;
mod gif;

use std::path::Path;
use std::io::Error;
use std::env;
use handler::handle_path;

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Terminal size reading failed");
    //let term_size = termsize::Size {rows: 50, cols: 50};
    let term_height = term_size.rows as usize;
    let term_width = term_size.cols as usize;
    println!("height: {term_height}, width: {term_width}");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [dirname/filename]"));
    }

    handle_path(Path::new(&args[1]), &term_size)
}
