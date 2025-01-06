pub mod bitmap {
    use std::{fs::File, io::Read};
    use std::io::{BufReader, BufRead};
    use std::io::{Error, Seek, SeekFrom};
    use std::fmt;
    
    use crate::common::common::{read_u16, read_u32, slice_to_usize_le};
    use crate::ansi::ansi::{erase_in_display, set_foreground_color, Erase, set_cursor_pos, Position};

    struct FileHeader {
        bfType: [u8; 2],
        bfSize: u32,
        bfReserved: u32,
        bfOffBits: u32
    }
    
    impl FileHeader {
        fn from_reader<R: BufRead>(reader: &mut R) -> std::io::Result<Self> {
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
        fn from_reader<R: BufRead>(reader: &mut R) -> std::io::Result<Self> {
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

    impl From<u32> for Color {
        fn from(value: u32) -> Self {
            let red = ((value >> 16) & 0xff) as u8;
            let green = ((value >> 8) & 0xff) as u8;
            let blue = (value & 0xff) as u8;
            Color {red, green, blue}
        }
    }

    impl Color {
        pub fn to_ansi(&self) -> String {
            set_foreground_color('█', self.to_string())
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

            let info_header = InfoHeader::from_reader(&mut reader)?;
            if info_header.biCompression != 0 {
                return Err(Error::other("Compressed Bitmap files not supported right now"));
            }
            
            let num_colortable_entries = match info_header.biBitCount {
                0..=8 => {
                    if info_header.biClrUsed == 0 {
                        2u32.pow(info_header.biBitCount.into())
                    } else {
                        info_header.biClrUsed
                    }
                },
                _ => 0
            };
            
            if file_header.bfOffBits < 54 + num_colortable_entries * 4 {
                return Err(Error::other("Pixel offset too small"));
            }

            println!("{file_header}");
            println!("{info_header}");

            println!("Start reading color table from offset: {}", reader.seek(SeekFrom::Current(0)).unwrap());
            
            let mut color_table = Vec::new();
            for _ in 0..num_colortable_entries {
                let argb = read_u32(&mut reader)?;
                color_table.push(Color::from(argb));
            }

            // Discard remaining bytes until start of pixel data
            let bytes_till_offset: usize = (file_header.bfOffBits - 54 - num_colortable_entries * 4) as usize;
            println!("Consume {bytes_till_offset} bytes after color table");
            reader.consume(bytes_till_offset);
            
            let height = info_header.biHeight.abs() as usize;
            let width = info_header.biWidth as usize;
            println!("Start reading pixels from offset: {}", reader.seek(SeekFrom::Current(0)).unwrap());
            let mut pixels = read_pixels(&mut reader, height, width, info_header.biBitCount, color_table)?;
            
            if info_header.biHeight > 0 {
                pixels.reverse();
            }

            Ok(Bitmap {width, height, pixels})
        }
        
        pub fn print(&self, term_height: usize, term_width: usize) {
            erase_in_display(Erase::SCREEN); 
            set_cursor_pos(Position {x: 1, y: 1});
            let mut picture = String::new();
            let y_step: f64 = (self.height as f64) / (term_height as f64);
            let x_step: f64 = (self.width as f64) / (term_width as f64);
            
            let mut fy: f64 = 0.0;
            for _ in 0..term_height {
                let y = fy.floor() as usize;
                let mut fx: f64 = 0.0;
                for _ in 0..term_width {
                    let x = fx.floor() as usize;
                    fx += x_step;
                    picture.push_str(&self.pixels[y][x].to_ansi());
                }
                fy += y_step;
                picture.push('\n');
            }

            print!("{picture}");
        }
    }

    fn read_pixels<R: Read + BufRead>(reader: &mut R, height: usize, width: usize, bits_per_pixel: u16, color_table: Vec<Color>) -> std::io::Result<Vec<Vec<Color>>> {
        let mut pixels = Vec::new();
        let (bytes_per_line, reads_per_line) = match bits_per_pixel {
            x @ (1 | 2 | 4 | 8) => (width, ((x as usize) * width)/8),
            24 => (width*3, width),
            32 => (width*4, width),
            _ => panic!("Not implemented yet")
        };
        let num_align_bytes = if bytes_per_line % 4 == 0 { 0 } else { 4 - (bytes_per_line % 4) };
        println!("Width: {width}, consume {num_align_bytes} bytes after each line");

        for _ in 0..height {
            let mut line = Vec::new();
            for _ in 0..reads_per_line {
                let res = match bits_per_pixel {
                    x @ (1 | 2 | 4 | 8) => read_colortable(reader, &color_table, x),
                    16 => Err(Error::other("Not implemented yet")),
                    24 => read_24bpp(reader),
                    32 => read_32bpp(reader),
                    _ => Err(Error::other("Not a valid bpp value"))
                };

                if let Err(err) = res {
                    println!("Could not read pixel values: {err}");
                    return Err(err);
                }

                line.append(&mut res.unwrap());
            }
            pixels.push(line);
            reader.consume(num_align_bytes);
        }

        Ok(pixels)
    }
    
    fn read_colortable<R: BufRead>(reader: &mut R, color_table: &Vec<Color>, bits_per_pixel: u16) -> std::io::Result<Vec<Color>> {
        let mut buf: [u8; 1] = [0; 1];
        reader.read_exact(&mut buf)?;
        let mut pixels = Vec::new();
        let start_shift = 8 - bits_per_pixel;
        let byte = buf[0] as usize;
        for i in 0..(8/bits_per_pixel) {
            let index: usize = (byte >> (start_shift - bits_per_pixel*i)) & (2usize.pow(bits_per_pixel as u32) - 1);
            if index > color_table.len() {
                return Err(Error::other("Out-of-bounds index"));
            }

            pixels.push(color_table[index]);
        }

        Ok(pixels)
    }
    
    fn read_24bpp<R: BufRead>(reader: &mut R) -> std::io::Result<Vec<Color>> {
        let mut rgb: [u8; 3] = [0; 3];
        reader.read_exact(&mut rgb)?;
        let argb = slice_to_usize_le(&mut rgb) as u32;
        Ok(vec![Color::from(argb)])
    }

    fn read_32bpp<R: BufRead>(reader: &mut R) -> std::io::Result<Vec<Color>> {
        let argb = read_u32(reader)?;
        Ok(vec![Color::from(argb)])
    }
}