use core::num;
use std::os::fd::AsRawFd;
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};
use std::fs::read_dir;
use std::env::current_dir;
use std::thread::current;

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

//impl PartialEq for FileInfo {
//    fn eq(&self, other: &Self) -> bool {
//        self.file_name != other.file_name ||
//        self.canon_name != other.canon_name ||
//        self.selected != other.selected
//    }
//
//    fn ne(&self, other: &Self) -> bool {
//        !self.eq(other)
//    }
//}

struct WindowMetadata {
    term_size: Size,
    header_reserved: usize,
    footer_reserved: usize,
    num_printable_lines: usize
}

struct DirState {
    path: PathBuf,
    name: String,
    files: Vec<FileInfo>
}

pub struct Window {
    metadata: WindowMetadata,
    dir_state: DirState,
    pos: CursorPos,
    page: Page,
    prev_termios: Termios
}

const HEADER_COLOR: Color = Color { red: 0xd5, green: 0x98, blue: 0x90 };
const FOOTER_COLOR: Color = Color { red: 0x23, green: 0x34, blue: 0x58 };
const SYMBOLS: [char; 4] = ['📄', '📁', '📂', '➜'];

impl Drop for Window {
    fn drop(&mut self) {
        self.restore_termios();
    }
}

impl Window {
    pub fn new() -> std::io::Result<Self> {
        let term_size = termsize::get().expect("Could not get terminal size");
        let term_height = term_size.rows as usize;
        let header_reserved = 2;
        let footer_reserved = 2;
        let total_reserved = header_reserved + footer_reserved;
        if term_height <= total_reserved + 1 {
            return Err(Error::other("Terminal not big enough to fit one file"));
        }
        let num_printable_lines = term_height - total_reserved;
        let metadata = WindowMetadata {term_size, header_reserved, footer_reserved, num_printable_lines};

        let path = current_dir()?;
        let Some(name) = path_to_string(&path) else {
            return Err(Error::other("Cannot get directory name"));
        };
        let files = Vec::with_capacity(num_printable_lines);
        let dir_state = DirState {path, name, files};

        let pos = CursorPos {x: 1, y: 3};
        let page = Page {x_page: 0, y_page: 0};
        let prev_termios = get_termios()?;
        Ok(Window {
            metadata,
            dir_state,
            pos,
            page,
            prev_termios
        })
    }

    // TODO: only update when button pressed
    pub fn do_interactive(&mut self) -> std::io::Result<()> {
        let mut writer = std::io::stdout();
        ansi::erase(Erase::SCREEN, &mut writer)?;
        self.update(&mut writer)?;

        loop {
            let Ok(input) = read_input() else {
                continue;
            };
            match input {
                b'w'  => self.move_up(&mut writer)?,
                b's'  => self.move_down(&mut writer)?,
                b'\n' => self.enter_dir()?,
                b'u'  => self.update(&mut writer)?,
                b'q'  => break,
                _     => continue
            }
            
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(())
    }

    fn read_current_dir(&mut self) -> std::io::Result<()> {
        self.update_term_size()?;

        let mut entries = read_dir(&self.dir_state.path)?;
        let mut files= Vec::with_capacity(self.metadata.num_printable_lines);

        // First entry on first page is parent if it exists
        if self.page.y_page == 0 {
            if let Some(parent_dir) = self.dir_state.path.parent() {
                let parent_info = FileInfo {
                    path: parent_dir.to_path_buf(),
                    file_name: String::from(".."),
                    canon_name: String::from("..")
                };
                files.push(parent_info);
            }
        }

        // Read max num_printable_lines directory entries
        for entry in entries {
            let dir_entry = entry?;
            let path = dir_entry.path();
            let Some(file_name) = path_to_string(&path) else {
                continue;
            };

            let canon = path.canonicalize()?;
            let Some(canon_name) = path_to_string(&canon) else {
                continue;
            };

            let file_info = FileInfo {
                path,
                file_name,
                canon_name
            };
            files.push(file_info);
        }
        
        //self.set_redraws(&mut files);
        self.dir_state.files = files;
        Ok(())
    }

    fn print_current_dir<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.print_header(writer, &self.dir_state.name)?;
 
        let mut line_index = 0;
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        for info in self.dir_state.files.iter().skip(entry_offset) {
            self.print_line(writer, info, line_index)?;

            line_index += 1;
        }
        ansi::erase(Erase::CURSOR_TO_END, writer)?;
        self.print_footer(writer)?;
        //self.select_current_line(writer)?;
        writer.flush()?;

        Ok(())
    }
    
    fn print_line<W: Write>(&self, writer: &mut W, info: &FileInfo, line_index: usize) -> std::io::Result<()> {
        ansi::erase(Erase::LINE, writer)?;

        let index = if info.path.is_file() { 0 } else { 1 };
        print!("{} ", SYMBOLS[index]);
        let mut text = String::from(&info.file_name);
        if info.path.is_symlink() {
            text.push_str(&format!(" {} {}", SYMBOLS[3], &info.canon_name));
        }
        self.print_highlighted_if(line_index, &text, writer)?;
        ansi::next_line(writer)?;

        Ok(())
    }

    fn print_header<W: Write>(&self, writer: &mut W, name: &str) -> std::io::Result<()> {
        ansi::reset_cursor(writer)?;

        // Print directory name
        let name= format!("{} {}", SYMBOLS[2], name);
        ansi::set_foreground_color(writer, &name, &HEADER_COLOR)?;
        ansi::next_line(writer)?;

        // Print divider
        let len = name.chars().count() + 2;
        let divider = String::from_iter(std::iter::repeat_n("-", len));
        ansi::set_foreground_color(writer, &divider, &HEADER_COLOR)?;
        ansi::next_line(writer)?;

        Ok(())
    }

    fn print_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let footer_start = self.metadata.header_reserved + self.metadata.num_printable_lines + 1;
        ansi::set_cursor(CursorPos {x: 1, y: footer_start}, writer)?;
        
        let page_text = format!("Page: {}", self.page.y_page);
        let len = page_text.chars().count() + 2;
        let divider = String::from_iter(std::iter::repeat('-').take(len));
        ansi::set_foreground_color(writer, &divider, &FOOTER_COLOR)?;
        ansi::next_line(writer)?;

        ansi::set_foreground_color(writer, &page_text, &FOOTER_COLOR)?;
        ansi::next_line(writer)?;

        Ok(())
    }
    
    fn print_highlighted_if<W: Write>(&self, line_index: usize, to_highlight: &str, writer: &mut W) -> std::io::Result<()> {
        let highlighted = line_index == self.pos_to_line_index(self.pos.y);
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

    fn move_up<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.pos.y -= 1;
        Ok(())
    }
    
    fn move_down<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.pos.y += 1;
        Ok(())
    }

    fn enter_dir(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn update<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.read_current_dir()?;
        self.print_current_dir(writer)
    }

    //fn select_current_line<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
    //    let line_index = self.pos_to_line_index(self.pos.y);
    //    if line_index >= self.dir_state.shown_entries.len() {
    //        panic!("Cursor points to non-existing entry");
    //    }

    //    let selected_entry = &self.dir_state.shown_entries[line_index];
    //    let symbol_overhead = 3;
    //    let x = if selected_entry.path.is_symlink() {
    //        selected_entry.file_name.chars().count() + selected_entry.canon_name.chars().count() + 2*symbol_overhead + 1
    //    } else {
    //        selected_entry.file_name.chars().count() + symbol_overhead + 1
    //    };
    //    
    //    self.pos.x = x;
    //    ansi::set_cursor(self.pos, writer)?;
    //    Ok(())
    //}

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
    
    pub fn restore_termios(&self) {
        let fd = std::io::stdin().as_raw_fd();
        if let Err(_) = tcsetattr(fd, TCSADRAIN, &self.prev_termios) {
            panic!("Could not restore tty attributes - cannot salvage");
        }
    }

    //fn set_redraws(&mut self, shown_entries: &mut Vec<FileInfo>) {
    //    // Don't redraw line unless changed or if needs highlighting
    //    for (line_index, (info_old, info_new)) in self.dir_state.shown_entries.iter().zip(shown_entries).enumerate() {
    //        if info_old == info_new && line_index != self.pos_to_line_index(self.pos.y) {
    //            info_new.redraw = false;
    //        }
    //    }
    //}

    fn update_term_size(&mut self) -> std::io::Result<()> {
        let term_size = termsize::get().expect("Could not get terminal size");
        if term_size.cols == self.metadata.term_size.cols &&
           term_size.rows == self.metadata.term_size.rows {
            return Ok(());
        }

        let term_height = term_size.rows as usize;
        let total_reserved = self.metadata.header_reserved + self.metadata.footer_reserved;
        if term_height <= total_reserved + 1 {
            return Err(Error::other("Terminal not big enough to fit one file"));
        }

        self.metadata.term_size = term_size;
        self.metadata.num_printable_lines = term_height - total_reserved;
        let last_line = self.metadata.header_reserved + self.metadata.num_printable_lines;
        self.pos.y = std::cmp::min(self.pos.y, last_line);
        Ok(())
    }

    fn pos_to_line_index(&self, y: usize) -> usize {
        if y <= self.metadata.header_reserved || y > self.metadata.header_reserved + self.metadata.num_printable_lines {
            panic!("CursorPosition Y out of bound");
        }
        
        y - self.metadata.header_reserved - 1
    }  
}

fn read_input() -> std::io::Result<u8> {
    let mut buf = [0; 1];
    std::io::stdin().read_exact(&mut buf)?;
    Ok(buf[0])
}

fn path_to_string(path: &Path) -> Option<String> {
    path.file_name().and_then(|os_filename| os_filename.to_str().and_then(|file_name| Some(String::from(file_name))))
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
