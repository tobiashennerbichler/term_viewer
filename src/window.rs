use core::num;
use std::os::fd::AsRawFd;
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};
use std::fs::read_dir;
use std::env::current_dir;
use std::thread::current;

use termios::{tcgetattr, tcsetattr, Termios, ICANON, ECHO, VMIN, TCSADRAIN};
use termsize::Size;
use crate::ansi::ansi::{self, Erase, CursorPos, Color, SGR};

const HEADER_RESERVED: usize = 2;
const FOOTER_RESERVED: usize = 3;
const HEADER_COLOR: Color  = Color { red: 0xd5, green: 0x98, blue: 0x90 };
const FOOTER_COLOR: Color  = Color { red: 0x23, green: 0x34, blue: 0x58 };
const ERROR_COLOR:  Color  = Color { red: 0xff, green: 0x08, blue: 0x4a };
const SYMBOLS: [char; 4] = ['📄', '📁', '📂', '➜'];

struct Page {
    x_page: usize,
    y_page: usize
}

struct FileInfo {
    path: PathBuf,
    file_name: String,
    canon_name: String,
    redraw: bool
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.file_name != other.file_name ||
        self.canon_name != other.canon_name
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

struct WindowMetadata {
    term_size: Size,
    printable_start: usize,
    footer_start: usize,
    num_printable_lines: usize,
    header_redraw: bool,
    footer_redraw: bool
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
    last_error: Option<String>,
    prev_termios: Termios
}

impl Drop for Window {
    fn drop(&mut self) {
        self.restore_termios();
    }
}

impl Window {
    pub fn new() -> std::io::Result<Self> {
        let term_size = termsize::get().ok_or(Error::other("Could not get terminal size"))?;
        let term_height = term_size.rows as usize;
        let total_reserved = HEADER_RESERVED + FOOTER_RESERVED;
        if term_height <= total_reserved + 1 {
            return Err(Error::other("Terminal not big enough to fit one file"));
        }

        let num_printable_lines = term_height - total_reserved;
        let printable_start = HEADER_RESERVED + 1;
        let footer_start = HEADER_RESERVED + num_printable_lines + 1;
        let metadata = WindowMetadata {
            term_size,
            printable_start,
            footer_start,
            num_printable_lines,
            header_redraw: true,
            footer_redraw: true
        };

        let path = current_dir()?;
        let Some(name) = path_to_string(&path) else {
            return Err(Error::other("Cannot get directory name"));
        };
        let files = Vec::with_capacity(num_printable_lines);
        let dir_state = DirState {path, name, files};

        let pos = CursorPos {x: 1, y: printable_start};
        let page = Page {x_page: 0, y_page: 0};
        let last_error = None;
        let prev_termios = get_termios()?;
        Ok(Window {
            metadata,
            dir_state,
            pos,
            page,
            last_error,
            prev_termios
        })
    }

    pub fn do_interactive(&mut self) -> std::io::Result<()> {
        let mut writer = std::io::stdout();
        ansi::erase(Erase::SCREEN, &mut writer)?;
        self.read_current_dir()?;
        self.print_current_dir(&mut writer)?;

        loop {
            self.update_term_size()?;
            if self.current_page_needs_redraw() {
                self.print_current_dir(&mut writer)?;
            }

            let Ok(input) = read_input() else {
                continue;
            };

            match input {
                b'w'  => self.move_up(&mut writer)?,
                b's'  => self.move_down(&mut writer)?,
                b'\n' => self.enter_dir()?,
                b'u'  => self.read_current_dir()?,
                b'q'  => break,
                _     => continue
            };
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
                    canon_name: String::from(".."),
                    redraw: true
                };
                files.push(parent_info);
            }
        }

        // Read max num_printable_lines directory entries
        for entry in entries {
            let Ok(dir_entry) = entry else {
                continue;
            };
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
                canon_name,
                redraw: true
            };
            files.push(file_info);
        }
        
        //self.clear_redraws_on_nochanges(&mut files);
        self.dir_state.files = files;
        Ok(())
    }

    //TODO: read_current_dir: snap cursor back to last entry on last page if new size shorter -> set redraws accordingly
    //also be mindful about resetting previously selected line
    //TODO: do update_term_size every frame, remove it from read_directory
    //TODO: update broken when resizing down
    //TODO: handle overflow on x-axis
    fn print_current_dir<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.print_header(writer)?;
 
        let mut y_pos = self.metadata.printable_start;
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        for info in self.dir_state.files.iter().skip(entry_offset).take(self.metadata.num_printable_lines) {
            if !info.redraw {
                ansi::next_line(writer)?;
                y_pos += 1;
                continue;
            }
            self.print_line(writer, info, y_pos)?;
            y_pos += 1;
        }

        self.clear_screen_to_footer(writer, y_pos)?;
        self.print_footer(writer)?;
        self.set_cursor_to_current_line_end(writer)?;
        writer.flush()?;

        self.set_entire_page_redraw(self.page.y_page, false);
        Ok(())
    }
    
    fn print_line<W: Write>(&self, writer: &mut W, info: &FileInfo, y_pos: usize) -> std::io::Result<()> {
        let index = if info.path.is_file() { 0 } else { 1 };
        let text = format!("{} ", SYMBOLS[index]);
        let mut htext = String::from(&info.file_name);
        if info.path.is_symlink() {
            htext.push_str(&format!(" {} {}", SYMBOLS[3], &info.canon_name));
        }
        
        write_highlight(writer, &text, &htext, y_pos == self.pos.y)?;
        Ok(())
    }

    fn print_header<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        if !self.metadata.header_redraw {
            ansi::set_cursor(CursorPos {x: 1, y: self.metadata.printable_start}, writer)?;
            return Ok(());
        }

        ansi::reset_cursor(writer)?;

        // Print directory name
        let name= format!("{} {}", SYMBOLS[2], self.dir_state.name);
        write_line(writer, &name, HEADER_COLOR)?;

        // Print divider
        let len = name.chars().count() + 3;
        let divider = String::from_iter(std::iter::repeat_n("-", len));
        write_line(writer, &divider, HEADER_COLOR)?;

        self.metadata.header_redraw = false;
        Ok(())
    }

    fn print_footer<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        if !self.metadata.footer_redraw {
            return Ok(());
        }

        ansi::set_cursor(CursorPos {x: 1, y: self.metadata.footer_start}, writer)?;
        let page_text = format!("Page: {}", self.page.y_page);
        let len = page_text.chars().count() + 2;
        let divider = String::from_iter(std::iter::repeat('-').take(len));
        write_line(writer, &divider, FOOTER_COLOR)?;
        write_line(writer, &page_text, FOOTER_COLOR)?;

        if let Some(ref error) = self.last_error {
            write_line(writer, &error, ERROR_COLOR)?;
        } else {
            ansi::erase(Erase::LINE, writer)?;
        }

        self.metadata.footer_redraw = false;
        Ok(())
    }
    
    fn clear_screen_to_footer<W: Write>(&self, writer: &mut W, mut y_pos: usize) -> std::io::Result<()> {
        while y_pos < self.metadata.footer_start {
            ansi::erase(Erase::LINE, writer)?;
            ansi::next_line(writer)?;
            y_pos += 1;
        }

        Ok(())
    }

    fn set_cursor_to_current_line_end<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        let line_index = self.pos_to_line_index(self.pos.y);
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        let file_index = line_index + entry_offset;

        let selected_entry = &self.dir_state.files[file_index];
        let symbol_overhead = 3;
        let x = if selected_entry.path.is_symlink() {
            selected_entry.file_name.chars().count() + selected_entry.canon_name.chars().count() + 2*symbol_overhead + 1
        } else {
            selected_entry.file_name.chars().count() + symbol_overhead + 1
        };
        
        self.pos.x = x;
        ansi::set_cursor(self.pos, writer)?;
        Ok(())
    }


    fn move_up<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        let line_index = self.pos_to_line_index(self.pos.y);
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        let file_index = line_index + entry_offset;
        if file_index == 0 {
            self.set_error(String::from("Error: Cannot move further up"));
            return Ok(());
        }

        if self.pos.y == self.metadata.printable_start {
            self.set_new_ypage(self.page.y_page - 1);
            self.pos.y = self.metadata.footer_start - 1;
            self.set_entire_page_redraw(self.page.y_page, true);
        } else {
            self.pos.y -= 1;
            self.dir_state.files[file_index].redraw = true;
            self.dir_state.files[file_index - 1].redraw = true;
        }

        self.clear_error();
        Ok(())
    }
    
    fn move_down<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        let line_index = self.pos_to_line_index(self.pos.y);
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        let file_index = line_index + entry_offset;
        if file_index == self.dir_state.files.len() - 1 {
            self.set_error(String::from("Error: Cannot move further down"));
            return Ok(());
        }

        if self.pos.y == self.metadata.footer_start - 1 {
            self.set_new_ypage(self.page.y_page + 1);
            self.pos.y = self.metadata.printable_start;
            self.set_entire_page_redraw(self.page.y_page, true);
        } else {
            self.pos.y += 1;
            self.dir_state.files[file_index].redraw = true;
            self.dir_state.files[file_index + 1].redraw = true;
        }

        self.clear_error();
        Ok(())
    }

    //TODO: set header_redraw
    fn enter_dir(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn set_new_ypage(&mut self, new_ypage: usize) {
        self.page.y_page = new_ypage;
        self.metadata.footer_redraw = true;
    }

    fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.metadata.footer_redraw = true;
    }

    fn clear_error(&mut self) {
        if self.last_error.is_some() {
            self.last_error = None;
            self.metadata.footer_redraw = true;
        }
    }

    fn current_page_needs_redraw(&self) -> bool {
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;

        self.dir_state.files.iter().skip(entry_offset).take(self.metadata.num_printable_lines).any(|file| file.redraw) |
        self.metadata.header_redraw | self.metadata.footer_redraw
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
    
    pub fn restore_termios(&self) {
        let fd = std::io::stdin().as_raw_fd();
        if let Err(_) = tcsetattr(fd, TCSADRAIN, &self.prev_termios) {
            panic!("Could not restore tty attributes - cannot salvage");
        }
    }

    fn clear_redraws_on_nochanges(&mut self, files: &mut Vec<FileInfo>) {
        // Don't redraw line unless changed or if needs highlighting
        for (line_index, (file_old, file_new)) in self.dir_state.files.iter().zip(files).enumerate() {
            if file_old == file_new && line_index != self.pos_to_line_index(self.pos.y) {
                file_new.redraw = false;
            }
        }
    }

    fn set_entire_page_redraw(&mut self, page: usize, redraw: bool) {
        let entry_offset = page * self.metadata.num_printable_lines;
        for file in self.dir_state.files.iter_mut().skip(entry_offset).take(self.metadata.num_printable_lines) {
            file.redraw = redraw;
        }
    }

    fn update_term_size(&mut self) -> std::io::Result<()> {
        let term_size = termsize::get().ok_or(Error::other("Could not get terminal size"))?;
        if term_size.cols == self.metadata.term_size.cols &&
           term_size.rows == self.metadata.term_size.rows {
            return Ok(());
        }

        let term_height = term_size.rows as usize;
        let total_reserved = HEADER_RESERVED + FOOTER_RESERVED;
        if term_height <= total_reserved + 1 {
            return Err(Error::other("Terminal not big enough to fit one file"));
        }
        
        let num_printable_lines = term_height - total_reserved;
        let footer_start = HEADER_RESERVED + num_printable_lines + 1;
        let line_index = self.pos_to_line_index(self.pos.y);
        let entry_offset = self.page.y_page * self.metadata.num_printable_lines;
        let file_offset = line_index + entry_offset;
        
        let new_line_index = file_offset % num_printable_lines;
        let new_y_page = file_offset / num_printable_lines;
            
        self.metadata.term_size = term_size;
        self.metadata.num_printable_lines = term_height - total_reserved;
        self.metadata.footer_start = footer_start;
        self.metadata.header_redraw = true;

        self.pos.y = new_line_index + self.metadata.printable_start;
        self.set_new_ypage(new_y_page);
        self.set_entire_page_redraw(new_y_page, true);
        Ok(())
    }

    fn pos_to_line_index(&self, y_pos: usize) -> usize {
        if y_pos < self.metadata.printable_start || y_pos >= self.metadata.footer_start {
            panic!("CursorPosition Y out of bound");
        }
        
        y_pos - self.metadata.printable_start
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

fn write_line<W: Write>(writer: &mut W, text: &str, color: Color) -> std::io::Result<()> {
    ansi::erase(Erase::LINE, writer)?;
    ansi::set_foreground_color(writer, text, color)?;
    ansi::next_line(writer)?;

    Ok(())
}

fn write_highlight<W: Write>(writer: &mut W, pretext: &str, htext: &str, highlight: bool) -> std::io::Result<()> {
    ansi::erase(Erase::LINE, writer)?;
    write!(writer, "{}", pretext)?;

    if highlight {
        ansi::set_sgr(SGR::FastBlink, writer)?;
        ansi::set_sgr(SGR::Underline, writer)?;
    }
    write!(writer, "{}", htext)?;
    if highlight {
        ansi::reset_sgr(writer)?;
    }

    ansi::next_line(writer)?;
    Ok(())
}