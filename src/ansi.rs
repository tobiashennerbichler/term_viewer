pub mod ansi {
    use std::fmt;
    use std::io::Write;

    const CSI: &str = "\x1b[";

    pub enum Erase {
        CURSOR_TO_END,
        CURSOR_TO_BEGIN,
        SCREEN,
        SCREEN_AND_DELETE
    }
    
    pub fn erase<W: Write>(mode: Erase, writer: &mut W) -> std::io::Result<()> {
        let n = match mode {
            Erase::CURSOR_TO_END => 0,
            Erase::CURSOR_TO_BEGIN => 1,
            Erase::SCREEN => 2,
            Erase::SCREEN_AND_DELETE => 3
        };
            
        write!(writer, "{CSI}{n}J")
    }

    #[derive(Copy, Clone, PartialEq)]
    pub struct Color {
        pub red: u8,
        pub green: u8,
        pub blue: u8
    }

    impl fmt::Debug for Color {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "r/g/b: {}/{}/{}", self.red, self.green, self.blue)
        }
    }

    impl From<u32> for Color {
        fn from(value: u32) -> Self {
            let red = ((value >> 16) & 0xff) as u8;
            let green = ((value >> 8) & 0xff) as u8;
            let blue = (value & 0xff) as u8;
            Color {red, green, blue}
        }
    }

    impl Color {
        pub fn print<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
            set_foreground_color(writer, '█', self.to_string())
        }

        fn to_string(&self) -> String {
            format!("{};{};{}", self.red, self.green, self.blue)
        }
    }

    fn set_foreground_color<W: Write>(writer: &mut W, character: char, color: String) -> std::io::Result<()> {
        write!(writer, "{CSI}38;2;{color}m{character}{CSI}m")
    }

    pub struct CursorPos {
        pub x: usize,
        pub y: usize
    }
    
    pub fn reset_cursor<W: Write>(writer: &mut W) -> std::io::Result<()> {
        set_cursor(CursorPos {x: 1, y: 1}, writer)
    }

    pub fn set_cursor<W: Write>(pos: CursorPos, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}{};{}H", pos.y, pos.x)
    }
        
    pub fn set_horizontal<W: Write>(x: usize, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}{}G", x)
    }
        
    pub fn next_line<W: Write>(writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}1E")
    }
}