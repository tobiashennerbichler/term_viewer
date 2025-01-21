mod bitmap;
mod common;
mod ansi;
mod handler;
mod gif;
mod window;

use std::path::Path;
use std::io::Error;
use std::env;
use handler::{handle_interactive, handle_path};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => handle_interactive(),
        2 => handle_path(Path::new(&args[1])),
        _ => Err(Error::other("Usage: cargo run -- [dirname/filename]"))
    }
}
