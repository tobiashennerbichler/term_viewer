mod bitmap;
mod common;
mod ansi;

use std::path::Path;
use std::io::Error;
use std::fs::read_dir;
use std::{env, thread};
use std::time::{Duration, Instant};

use bitmap::bitmap::Bitmap;

const MILLIS_PER_FRAME: u64 = 33;
const DURATION_PER_FRAME: Duration = Duration::from_millis(MILLIS_PER_FRAME);

fn handle_dir(path: &Path, term_height: usize, term_width: usize) -> std::io::Result<()> {
    let entries = read_dir(path)?;
    let mut prev = None;
    for entry in entries {
        let start = Instant::now();
        let dir_entry = entry?;
        let entry_path = dir_entry.path();
        let entry_metadata = entry_path.metadata()?;
        if !entry_metadata.is_file() {
            continue;
        }

        let curr_bitmap = handle_file(&entry_path, term_height, term_width, prev)?;
        let end = Instant::now();
        let time_spent = end.duration_since(start);
        if let Some(remaining_time) = DURATION_PER_FRAME.checked_sub(time_spent) {
            thread::sleep(remaining_time);
        }
        prev = Some(curr_bitmap);
    }

    Ok(())
}

fn handle_file(path: &Path, term_height: usize, term_width: usize, prev: Option<Bitmap>) -> std::io::Result<Bitmap> {
    let bitmap = Bitmap::new(path)?;
    bitmap.print(term_height, term_width, prev)?;
    Ok(bitmap)
}

fn main() -> std::io::Result<()> {
    let term_size = termsize::get().expect("Should not fail");
    let term_height = term_size.rows as usize;
    let term_width = term_size.cols as usize;
    println!("height: {term_height}, width: {term_width}");
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::other("Usage: cargo run -- [dirname/filename]"));
    }
    
    let path = Path::new(&args[1]);
    let metadata = path.metadata()?;
    if metadata.is_dir() {
        handle_dir(&path, term_height, term_width)
    } else {
        handle_file(&path, term_height, term_width, None)?;
        Ok(())
    }
}
