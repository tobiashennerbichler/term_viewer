use std::thread;
use std::path::Path;
use std::fs::read_dir;
use std::io::Error;
use std::time::{Duration, Instant};
use std::ffi::OsStr;

use termsize::Size;
use crate::bitmap::bitmap::Bitmap;
use crate::gif::gif::Gif;
use crate::window::Window;

const MILLIS_PER_FRAME: u64 = 33;
const DURATION_PER_FRAME: Duration = Duration::from_millis(MILLIS_PER_FRAME);


pub fn handle_interactive() -> std::io::Result<()> {
    let mut window = Window::new()?;
    window.setup_termios()?;
    window.do_interactive()
}

pub fn handle_path(path: &Path) -> std::io::Result<()> {
    let term_size = termsize::get().expect("Could not get termsize");
    let metadata = path.metadata()?;
    if metadata.is_dir() {
        handle_dir(&path, &term_size)
    } else {
        handle_file(&path, &term_size)
    }
}

fn handle_dir(path: &Path, term_size: &Size) -> std::io::Result<()> {
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

        if entry_path.extension().is_none_or(|ext| ext != OsStr::new("bmp")) {
            continue;
        }

        let curr_bitmap = handle_bitmap(&entry_path, term_size, prev)?;
        let end = Instant::now();
        let time_spent = end.duration_since(start);
        if let Some(remaining_time) = DURATION_PER_FRAME.checked_sub(time_spent) {
            thread::sleep(remaining_time);
        }
        prev = Some(curr_bitmap);
    }

    Ok(())
}

fn handle_file(path: &Path, term_size: &Size) -> std::io::Result<()> {
    let Some(extension) = path.extension() else {
        return Err(Error::other("No file extension"));
    };

    match extension.to_str().unwrap() {
        "bmp" => handle_bitmap(path, term_size, None).and(Ok(())),
        "gif" => handle_gif(path, term_size),
        _ => Err(Error::other("Not a supported extension"))
    }
}

fn handle_bitmap(path: &Path, term_size: &Size, prev: Option<Bitmap>) -> std::io::Result<Bitmap> {
    let bitmap = Bitmap::new(path)?;
    bitmap.print(term_size, prev)?;
    Ok(bitmap)
}

fn handle_gif(path: &Path, term_size: &Size) -> std::io::Result<()> {
    let mut gif = Gif::new(path)?;
    gif.print(term_size)
}