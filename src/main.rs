mod bitmap;
mod common;
mod ansi;

use std::path::Path;
use std::io::Error;
use std::fs::read_dir;
use std::{env, thread, time};

use bitmap::bitmap::Bitmap;

const MILLIS_PER_FRAME: u64 = 33;

fn handle_dir(path: &Path, term_height: usize, term_width: usize) -> std::io::Result<()> {
    let entries = read_dir(path)?;
    for entry in entries {
        let dir_entry = entry?;
        let entry_path = dir_entry.path();
        let entry_metadata = entry_path.metadata()?;

        if !entry_metadata.is_file() {
            continue;
        }

        match handle_file(&entry_path, term_height, term_width) {
            Ok(_) => thread::sleep(time::Duration::from_millis(MILLIS_PER_FRAME)),
            Err(err) => {
                println!("Could not read file {entry_path:?}: {err}");
                continue;
            }
        }
    }

    Ok(())
}

fn handle_file(path: &Path, term_height: usize, term_width: usize) -> std::io::Result<()> {
    let Some(extension) = path.extension() else {
        return Err(Error::other("No file extension"));
    };

    if extension != "bmp" {
        return Err(Error::other("Not a Bitmap file extension"));
    }
    
    let bitmap = Bitmap::new(path)?;
    bitmap.print(term_height, term_width);

    Ok(())
}

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Should not fail");
    let term_height = term_size.rows as usize;
    let term_width = term_size.cols as usize;
    println!("height: {term_height}, width: {term_width}");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [dirname]"));
    }
    
    let path = Path::new(&args[1]);
    let metadata = path.metadata()?;
    if metadata.is_dir() {
        handle_dir(&path, term_height, term_width)
    } else {
        handle_file(&path, term_height, term_width)
    }
}
