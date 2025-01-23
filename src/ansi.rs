pub mod ansi {
    use std::fmt;
    use std::io::Write;

    const CSI: &str = "\x1b[";

    pub enum Erase {
        CURSOR_TO_END,
        CURSOR_TO_BEGIN,
        SCREEN,
        SCREEN_AND_DELETE,
        CURSOR_TO_LINE_END,
        CURSOR_TO_LINE_BEGIN,
        LINE
    }
    
    pub fn erase<W: Write>(mode: Erase, writer: &mut W) -> std::io::Result<()> {
        let code = match mode {
            Erase::CURSOR_TO_END | 
            Erase::CURSOR_TO_BEGIN |
            Erase::SCREEN |
            Erase::SCREEN_AND_DELETE => 'J',
            Erase::CURSOR_TO_LINE_END |
            Erase::CURSOR_TO_LINE_BEGIN |
            Erase::LINE => 'K'
        };

        let n = match mode {
            Erase::CURSOR_TO_END | Erase::CURSOR_TO_LINE_END => 0,
            Erase::CURSOR_TO_BEGIN | Erase::CURSOR_TO_LINE_BEGIN => 1,
            Erase::SCREEN | Erase::LINE => 2,
            Erase::SCREEN_AND_DELETE => 3,
        };
            
        write!(writer, "{CSI}{n}{code}")
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
            set_foreground_color(writer, "█", *self)
        }

        fn to_string(&self) -> String {
            format!("{};{};{}", self.red, self.green, self.blue)
        }
    }

    pub fn set_foreground_color<W: Write>(writer: &mut W, text: &str, color: Color) -> std::io::Result<()> {
        write!(writer, "{CSI}38;2;{}m{text}{CSI}m", color.to_string())
    }

    #[derive(Copy, Clone)]
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

    pub fn cursor_forward<W: Write>(step: usize, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}{step}C")
    }
        
    pub fn set_horizontal<W: Write>(x: usize, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}{}G", x)
    }
        
    pub fn next_line<W: Write>(writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}1E")
    }

    pub enum SGR {
        Underline,
        FastBlink
    }

    pub fn set_sgr<W: Write>(mode: SGR, writer: &mut W) -> std::io::Result<()> {
        let n = match mode {
            SGR::Underline => 4,
            SGR::FastBlink => 6
        };
        
        write!(writer, "{CSI}{n}m")
    }

    pub fn reset_sgr<W: Write>(writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{CSI}0m")
    }
}