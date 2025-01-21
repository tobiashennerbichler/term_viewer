use core::num;
use std::os::fd::AsRawFd;
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};
use std::fs::read_dir;
use std::env::current_dir;

use termios::{tcgetattr, tcsetattr, Termios, ICANON, ECHO, VMIN, TCSADRAIN};
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
    top_reserved: usize,
    bottom_reserved: usize,
    num_printable_lines: usize,
    pos: CursorPos,
    page: Page,
    dir_name: String,
    current_dir_state: Vec<FileInfo>,
    prev_termios: Termios
}

const HEADER_COLOR: Color = Color { red: 0xd5, green: 0x98, blue: 0x90 };
const SYMBOLS: [char; 4] = ['📄', '📁', '📂', '➜'];

impl Window {
    pub fn new(term_size: Size) -> std::io::Result<Self> {
        let term_height = term_size.rows as usize;
        let top_reserved = 2;
        let bottom_reserved = 2;
        if term_height <= top_reserved + bottom_reserved {
            return Err(Error::other("Terminal not big enough"));
        }

        let num_printable_lines = term_height - top_reserved - bottom_reserved;
        let pos = CursorPos {x: 1, y: 3};
        let page = Page {x_page: 0, y_page: 0};
        let dir_name = String::new();
        let current_dir_state = Vec::with_capacity(num_printable_lines);
        let prev_termios = get_termios()?;
        Ok(Window {term_size, top_reserved, bottom_reserved, num_printable_lines, pos, page, dir_name, current_dir_state, prev_termios})
    }

    pub fn do_interactive(&mut self) -> std::io::Result<()> {
        let mut writer = std::io::stdout();
        loop {
            self.read_current_dir()?;
            self.print_current_dir(&mut writer)?;
        
            let mut buf = [0; 1];
            if let Err(_) = std::io::stdin().read_exact(&mut buf) {
               continue; 
            }
            match buf[0] {
                b'w' => self.pos.y -= 1,
                b's' => self.pos.y += 1,
                b'q' => break,
                _ => continue
            }
            self.select_current_line(&mut writer)?;
            
            std::thread::sleep(std::time::Duration::from_millis(100));
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

        // First entry is parent dir if exists
        if let Some(parent_dir) = current_dir.parent() {
            let parent_info = FileInfo {path: parent_dir.to_path_buf(), file_name: String::from(".."), canon_name: String::from("..")};
            dir_state.push(parent_info);
        }

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

    fn print_current_dir<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let entry_offset = self.page.x_page * self.num_printable_lines;

        self.print_header(writer, &self.dir_name)?;
        let infos = self.current_dir_state.iter().skip(entry_offset).take(self.num_printable_lines);
 
        let mut counter = self.top_reserved + 1;
        for info in infos {
            let metadata = info.path.metadata()?;
            let index = if metadata.is_file() { 0 } else { 1 };

            print!("{} ", SYMBOLS[index]);
            self.print_highlighted_if(counter, &info.file_name, writer)?;
            if metadata.is_symlink() {
                print!(" {} {}", SYMBOLS[3], &info.canon_name);
            }
            ansi::next_line(writer)?;

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
            ansi::reset_sgr(writer)?;
        }

        Ok(())
    }

    fn select_current_line<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        if self.pos.y <= self.top_reserved || self.pos.y >= self.num_printable_lines {
            self.restore_termios()?;
            panic!("Cursor should not be on reserved lines");
        }

        let entry_index = self.pos.y - self.top_reserved - 1;
        if entry_index > self.current_dir_state.len() {
            self.restore_termios()?;
            panic!("Cursor points to non-existing entry");
        }
        let selected_entry = &self.current_dir_state[entry_index];
        let symbol_overhead = 3;
        let x = if selected_entry.path.is_symlink() {
            selected_entry.file_name.chars().count() + selected_entry.file_name.chars().count() + 2*symbol_overhead + 1
        } else {
            selected_entry.file_name.chars().count() + symbol_overhead + 1
        };
        
        self.pos.x = x;
        ansi::set_cursor(self.pos, writer)?;
        Ok(())
    }

    pub fn setup_termios(&self) -> std::io::Result<()> {
        let fd = std::io::stdin().as_raw_fd();
        let Ok(mut termios) = Termios::from_fd(fd) else {
            return Err(Error::other("Could not create termios from stdin"));
        };

        termios.c_lflag &= !(ICANON | ECHO);
        termios.c_cc[VMIN] = 0;
        if let Err(_) = tcsetattr(fd, TCSADRAIN, &termios) {
            return Err(Error::other("Could not set tty attributes"));
        }
    
        Ok(())
    }
    
    pub fn restore_termios(&self) -> std::io::Result<()> {
        let fd = std::io::stdin().as_raw_fd();
        if let Err(_) = tcsetattr(fd, TCSADRAIN, &self.prev_termios) {
            return Err(Error::other("Could not restore tty attributes"));
        }

        Ok(())
    }
}

fn path_to_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|os_filename| os_filename.to_str())
}

fn get_termios() -> std::io::Result<Termios> {
    let fd = std::io::stdin().as_raw_fd();
    let Ok(mut termios) = Termios::from_fd(fd) else {
        return Err(Error::other("Cannot create termios from stdin"));
    };

    if let Err(_) = tcgetattr(fd, &mut termios) {
        return Err(Error::other("Could not get tty attributes"));
    }

    Ok(termios)
}
