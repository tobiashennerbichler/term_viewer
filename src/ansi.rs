pub mod ansi {
    use std::fmt;

    const CSI: &str = "\x1b[";

    pub enum Erase {
        CURSOR_TO_END,
        CURSOR_TO_BEGIN,
        SCREEN,
        SCREEN_AND_DELETE
    }

    impl Erase {
        pub fn erase(&self) {
            let n = match self {
                Erase::CURSOR_TO_END => 0,
                Erase::CURSOR_TO_BEGIN => 1,
                Erase::SCREEN => 2,
                Erase::SCREEN_AND_DELETE => 3
            };
            
            print!("{CSI}{n}J");
        }
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
        pub fn print(&self) {
            set_foreground_color('█', self.to_string());
        }

        fn to_string(&self) -> String {
            format!("{};{};{}", self.red, self.green, self.blue)
        }
    }

    fn set_foreground_color(character: char, color: String) {
        print!("{CSI}38;2;{color}m{character}{CSI}m")
    }

    pub struct CursorPos {
        pub x: usize,
        pub y: usize
    }
    
    impl CursorPos {
        pub fn reset_cursor() {
            CursorPos {x: 1, y: 1}.set_cursor();
        }

        pub fn set_cursor(&self) {
            print!("{CSI}{};{}H", self.y, self.x);
        }
        
        pub fn set_horizontal(x: usize) {
            print!("{CSI}{}G", x);
        }
        
        pub fn next_line() {
            print!("{CSI}1E");
        }
    }
}