pub mod ansi {
    const CSI: &str = "\x1b[";

    pub enum Erase {
        CURSOR_TO_END,
        CURSOR_TO_BEGIN,
        SCREEN,
        SCREEN_AND_DELETE
    }

    pub fn erase_in_display(mode: Erase) {
        let n = match mode {
            Erase::CURSOR_TO_END => 0,
            Erase::CURSOR_TO_BEGIN => 1,
            Erase::SCREEN => 2,
            Erase::SCREEN_AND_DELETE => 3
        };

        print!("{CSI}{n}J");
    }
    
    pub fn set_foreground_color(character: char, color: String) {
        print!("{CSI}38;2;{color}m{character}{CSI}m")
    }

    pub struct Position {
        pub x: usize,
        pub y: usize
    }

    pub fn set_cursor_pos(pos: Position) {
        print!("{CSI}{};{}H", pos.y, pos.x);
    }
}