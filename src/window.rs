use core::num;
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use std::fs::read_dir;
use std::env::current_dir;

use termsize::Size;
use crate::ansi::ansi::{self, Erase, CursorPos, Color};

struct Page {
    x_page: usize,
    y_page: usize
}

struct FileInfo {
    path: PathBuf,
    file_name: String,
    canon_name: String
}

pub struct Window {
    term_size: Size,
    reserved_lines: usize,
    num_printable_lines: usize,
    pos: CursorPos,
    page: Page,
    dir_name: String,
    current_dir_state: Vec<FileInfo>
}

const HEADER_COLOR: Color = Color { red: 0xd5, green: 0x98, blue: 0x90 };
const SYMBOLS: [char; 4] = ['📄', '📁', '📂', '➜'];

impl Window {
    pub fn new(term_size: Size) -> std::io::Result<Self> {
        let term_height = term_size.rows as usize;
        let reserved_lines = 4;
        if term_height <= reserved_lines {
            return Err(Error::other("Terminal not big enough"));
        }
        let num_printable_lines = term_height - reserved_lines;

        let pos = CursorPos {x: 1, y: 6};
        let page = Page {x_page: 0, y_page: 0};
        let dir_name = String::new();
        let current_dir_state = Vec::with_capacity(num_printable_lines);
        Ok(Window {term_size, reserved_lines, num_printable_lines, pos, page, dir_name, current_dir_state})
    }

    pub fn do_interactive(&mut self) -> std::io::Result<()> {
        loop {
            self.read_current_dir()?;
            self.print_current_dir()?;
            
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        Ok(())
    }

    fn read_current_dir(&mut self) -> std::io::Result<()> {
        let current_dir = current_dir()?;
        let entries = read_dir(&current_dir)?;
        let mut dir_state = Vec::with_capacity(self.num_printable_lines);

        let Some(dir_name) = path_to_str(&current_dir) else {
            return Err(Error::other("Could not convert current dirname to str"));
        };
        let dir_name = String::from(dir_name);

        for entry in entries {
            let dir_entry = entry?;
            let path = dir_entry.path();

            let Some(filename) = path_to_str(&path) else {
                continue;
            };
            let file_name = String::from(filename);

            let canon = path.canonicalize()?;
            let Some(canon_name) = path_to_str(&canon) else {
                continue;
            };
            let canon_name = String::from(canon_name);
            
            let file_info = FileInfo {path, file_name, canon_name};
            dir_state.push(file_info);
        }

        self.dir_name = dir_name;
        self.current_dir_state = dir_state;
        Ok(())
    }

    fn print_current_dir(&self) -> std::io::Result<()> {
        let mut writer = std::io::stdout();
        let entry_offset = self.page.x_page * self.num_printable_lines;

        let infos = self.current_dir_state.iter().skip(entry_offset).take(self.num_printable_lines);
        self.print_header(&mut writer, &self.dir_name)?;
 
        let mut counter = 2;
        for info in infos {
            let metadata = info.path.metadata()?;

            let index = if metadata.is_file() { 0 } else { 1 };

            print!("{} ", SYMBOLS[index]);
            self.print_highlighted_if(counter, &info.file_name, &mut writer)?;
            if metadata.is_symlink() {
                print!(" {} {}", SYMBOLS[3], &info.canon_name);
            }
            ansi::next_line(&mut writer)?;

            counter += 1;
        }
        writer.flush()?;

        Ok(())
    }

    fn print_header<W: Write>(&self, writer: &mut W, dir_name: &str) -> std::io::Result<()> {
        ansi::erase(Erase::SCREEN, writer)?;
        ansi::reset_cursor(writer)?;

        // Print directory name
        let dir_name= format!("{} {}", SYMBOLS[2], dir_name);
        ansi::set_foreground_color(writer, &dir_name, &HEADER_COLOR)?;
        ansi::next_line(writer)?;

        // Print divider
        let len = dir_name.chars().count() + 2;
        let divider = String::from_iter(std::iter::repeat_n("-", len));
        ansi::set_foreground_color(writer, &divider, &HEADER_COLOR)?;
        ansi::next_line(writer)?;

        // Print ".." path
        print!("{} ", SYMBOLS[1]);
        self.print_highlighted_if(1, "..", writer)?;
        ansi::next_line(writer)?;
        
        Ok(())
    }
    
    fn print_highlighted_if<W: Write>(&self, y_pos: usize, to_highlight: &str, writer: &mut W) -> std::io::Result<()> {
        let highlighted = y_pos == self.pos.y;
        if highlighted {
            ansi::make_fast_blinking(writer)?;
            ansi::make_underline(writer)?;
        }
        print!("{to_highlight}");
        
        if highlighted {
            ansi::reset_SGR(writer)?;
        }

        Ok(())
    }
}

fn path_to_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|os_filename| os_filename.to_str())
}
