pub mod bitmap {
    use std::{fs::File, io::Read};
    use std::io::{BufReader, BufRead};
    use std::io::Error;
    use std::fmt;
    
    use crate::common::common::{read_u16, read_u32};
    use crate::ansi::ansi::{erase_in_display, set_foreground_color, Erase, set_cursor_pos, Position};

    struct FileHeader {
        bfType: [u8; 2],
        bfSize: u32,
        bfReserved: u32,
        bfOffBits: u32
    }
    
    impl FileHeader {
        fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
            let mut bfType = [0; 2];
            reader.read_exact(&mut bfType)?;
            let bfSize = read_u32(reader)?;
            let bfReserved = read_u32(reader)?;
            let bfOffBits = read_u32(reader)?;

            Ok(FileHeader {
                bfType,
                bfSize,
                bfReserved,
                bfOffBits
            })
        }
    }

    impl fmt::Display for FileHeader {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "FILEHEADER:\n\ttype: {:?}\n\tfilesize: {}\n\toffset: {}", self.bfType, self.bfSize, self.bfOffBits)
        }
    }

    struct InfoHeader {
        biSize: u32,
        biWidth: i32,
        biHeight: i32,
        biPlanes: u16,
        biBitCount: u16,
        biCompression: u32,
        biSizeImage: u32,
        biXPelsPerMeter: i32,
        biYPelsPerMeter: i32,
        biClrUsed: u32,
        biClrImportant: u32
    }
    
    impl InfoHeader {
        fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
            let biSize = read_u32(reader)?;
            let biWidth = read_u32(reader)? as i32;
            let biHeight = read_u32(reader)? as i32;
            let biPlanes = read_u16(reader)?;
            let biBitCount = read_u16(reader)?;
            let biCompression = read_u32(reader)?;
            let biSizeImage = read_u32(reader)?;
            let biXPelsPerMeter = read_u32(reader)? as i32;
            let biYPelsPerMeter = read_u32(reader)? as i32;
            let biClrUsed = read_u32(reader)?;
            let biClrImportant = read_u32(reader)?;

            Ok(InfoHeader {
                biSize,
                biWidth,
                biHeight,
                biPlanes,
                biBitCount,
                biCompression,
                biSizeImage,
                biXPelsPerMeter,
                biYPelsPerMeter,
                biClrUsed,
                biClrImportant
            })
        }
    }

    impl fmt::Display for InfoHeader {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "INFOHEADER:\n\tinfoheader size: {}\n\twidth: {}\n\theight: {}\n\tdepth: {}\n\tcompression: {}\n\timagesize: {} \
            \n\tclrused: {}\n\tclrimportant: {}", self.biSize, self.biWidth, self.biHeight, self.biBitCount, self.biCompression,
            self.biSizeImage, self.biClrUsed, self.biClrImportant)
        }
    }

    #[derive(Copy, Clone)]
    pub struct Color {
        red: u8,
        green: u8,
        blue: u8
    }

    impl TryFrom<&[u8]> for Color {
        type Error = &'static str;

        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != 3 {
                return Err("Slice too long");
            }

            Ok(Color {red: value[2], green: value[1], blue: value[0]})
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

    pub struct Bitmap {
        pub width: usize,
        pub height: usize,
        pub pixels: Vec<Vec<Color>>
    }
    
    impl Bitmap {
        pub fn new(filename: &str) -> std::io::Result<Self> {
            let file = File::open(filename)?;
            let mut reader = BufReader::new(file);

            let file_header = FileHeader::from_reader(&mut reader)?;
            if &file_header.bfType != b"BM" {
                return Err(Error::other("File does not start with Bitmap magic values"));
            }
            if file_header.bfOffBits < 54 {
                return Err(Error::other("Offset too small"));
            }

            let info_header = InfoHeader::from_reader(&mut reader)?;
            if info_header.biBitCount != 24 {
                return Err(Error::other("Bitdepth other than 24 not supported right now"));
            }
            if info_header.biCompression != 0 {
                return Err(Error::other("Compressed Bitmap files not supported right now"));
            }

            // Consume possible bytes until pixel table starts
            let bytes_till_offset = (file_header.bfOffBits - 54) as usize;
            reader.consume(bytes_till_offset);

            println!("{file_header}");
            println!("{info_header}");
            let height = info_header.biHeight.abs() as usize;
            let width = info_header.biWidth as usize;
            let mut pixels = read_pixels(&mut reader, height, width)?;
            
            if info_header.biHeight > 0 {
                pixels.reverse();
            }

            Ok(Bitmap {width, height, pixels})
        }
        
        pub fn print(&self, term_height: usize, term_width: usize) {
            erase_in_display(Erase::SCREEN); 
            set_cursor_pos(Position {x: 1, y: 1});
            let y_step = std::cmp::max(self.height / term_height, 1);
            let x_step = std::cmp::max(self.width / term_width, 1);
            let height = std::cmp::min(self.height, term_height);
            let width = std::cmp::min(self.width, term_width);
            
            for y in 0..height {
                for x in 0..width {
                    self.pixels[y*y_step][x*x_step].print();
                }
                println!("");
            }
        }
    }

    fn read_pixels(reader: &mut BufReader<File>, height: usize, width: usize) -> std::io::Result<Vec<Vec<Color>>> {
        let mut pixels = Vec::new();
        let num_align_bytes = (width*3) % 4;

        for y in 0..height {
            let mut line = Vec::new();
            for x in 0..width {
                let mut rgb: [u8; 3] = [0; 3];
                if let Err(err) = reader.read_exact(&mut rgb) {
                    println!("Could not read 3 bytes: {err}");
                    return Err(err);
                }

                line.push(Color::try_from(&rgb[..]).unwrap());
            }
            pixels.push(line);
            reader.consume(num_align_bytes);
        }

        Ok(pixels)
    }
}