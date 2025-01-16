pub mod bitmap {
    use std::{fs::File, io::Read};
    use std::io::{BufReader, BufRead};
    use std::io::{Error, Seek, SeekFrom};
    use std::path::Path;
    use std::fmt;
    
    use crate::common::common::{read_u16, read_u32, slice_to_usize_le};
    use crate::ansi::ansi::{erase_in_display, set_foreground_color, Erase, set_cursor_pos, Position};

    struct FileHeader {
        bf_type: [u8; 2],
        bf_size: u32,
        bf_reserved: u32,
        bf_off_bits: u32
    }
    
    impl FileHeader {
        fn from_reader<R: BufRead>(reader: &mut R) -> std::io::Result<Self> {
            let mut bf_type = [0; 2];
            reader.read_exact(&mut bf_type)?;
            let bf_size = read_u32(reader)?;
            let bf_reserved = read_u32(reader)?;
            let bf_off_bits = read_u32(reader)?;

            Ok(FileHeader {
                bf_type,
                bf_size,
                bf_reserved,
                bf_off_bits
            })
        }
    }

    impl fmt::Display for FileHeader {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "FILEHEADER:\n\ttype: {:?}\n\tfilesize: {}\n\toffset: {}", self.bf_type, self.bf_size, self.bf_off_bits)
        }
    }

    struct InfoHeader {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32
    }
    
    impl InfoHeader {
        fn from_reader<R: BufRead>(reader: &mut R) -> std::io::Result<Self> {
            let bi_size = read_u32(reader)?;
            let bi_width = read_u32(reader)? as i32;
            let bi_height = read_u32(reader)? as i32;
            let bi_planes = read_u16(reader)?;
            let bi_bit_count = read_u16(reader)?;
            let bi_compression = read_u32(reader)?;
            let bi_size_image = read_u32(reader)?;
            let bi_x_pels_per_meter = read_u32(reader)? as i32;
            let bi_y_pels_per_meter = read_u32(reader)? as i32;
            let bi_clr_used = read_u32(reader)?;
            let bi_clr_important = read_u32(reader)?;

            Ok(InfoHeader {
                bi_size,
                bi_width,
                bi_height,
                bi_planes,
                bi_bit_count,
                bi_compression,
                bi_size_image,
                bi_x_pels_per_meter,
                bi_y_pels_per_meter,
                bi_clr_used,
                bi_clr_important
            })
        }
    }

    impl fmt::Display for InfoHeader {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "INFOHEADER:\n\tinfoheader size: {}\n\twidth: {}\n\theight: {}\n\tdepth: {}\n\tcompression: {}\n\timagesize: {} \
            \n\tclrused: {}\n\tclrimportant: {}", self.bi_size, self.bi_width, self.bi_height, self.bi_bit_count, self.bi_compression,
            self.bi_size_image, self.bi_clr_used, self.bi_clr_important)
        }
    }

    #[derive(Copy, Clone)]
    pub struct Color {
        red: u8,
        green: u8,
        blue: u8
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

    pub struct Bitmap {
        pub width: usize,
        pub height: usize,
        pub pixels: Vec<Vec<Color>>
    }
    
    impl Bitmap {
        pub fn new(path: &Path) -> std::io::Result<Self> {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);

            let file_header = FileHeader::from_reader(&mut reader)?;
            if &file_header.bf_type != b"BM" {
                return Err(Error::other("File does not start with Bitmap magic values"));
            }

            let info_header = InfoHeader::from_reader(&mut reader)?;
            if info_header.bi_compression != 0 {
                return Err(Error::other("Compressed Bitmap files not supported right now"));
            }
            
            let num_colortable_entries = match info_header.bi_bit_count {
                1 | 2 | 4 | 8 => {
                    if info_header.bi_clr_used == 0 {
                        2u32.pow(info_header.bi_bit_count.into())
                    } else {
                        info_header.bi_clr_used
                    }
                },
                16 | 24 | 32 => 0,
                _ => return Err(Error::other("Not a valid bpp value"))
            };
            
            if file_header.bf_off_bits < 54 + num_colortable_entries * 4 {
                return Err(Error::other("Pixel offset too small"));
            }

            //println!("{file_header}");
            //println!("{info_header}");

            let mut color_table = Vec::new();
            for _ in 0..num_colortable_entries {
                let argb = read_u32(&mut reader)?;
                color_table.push(Color::from(argb));
            }

            // Discard remaining bytes until start of pixel data
            let bytes_till_offset: usize = (file_header.bf_off_bits - 54 - num_colortable_entries * 4) as usize;
            reader.consume(bytes_till_offset);
            
            let height = info_header.bi_height.abs() as usize;
            let width = info_header.bi_width as usize;
            let mut pixels = read_pixels(&mut reader, height, width, info_header.bi_bit_count, color_table)?;
            
            // Transform bottom-up to top-down
            if info_header.bi_height > 0 {
                pixels.reverse();
            }

            Ok(Bitmap {width, height, pixels})
        }
        
        pub fn print(&self, term_height: usize, term_width: usize) {
            erase_in_display(Erase::SCREEN); 
            set_cursor_pos(Position {x: 1, y: 1});
            let y_step: f64 = f64::max((self.height as f64) / (term_height as f64), 1.0);
            let x_step: f64 = f64::max((self.width as f64) / (term_width as f64), 1.0);
            let height = std::cmp::min(self.height, term_height);
            let width = std::cmp::min(self.width, term_width);
            
            let mut fy: f64 = 0.0;
            for _ in 0..height {
                let y = fy.floor() as usize;
                let mut fx: f64 = 0.0;
                for _ in 0..width {
                    let x = fx.floor() as usize;
                    fx += x_step;
                    self.pixels[y][x].print();
                }
                fy += y_step;
                println!("");
            }
        }
    }

    fn read_pixels<R: Read + BufRead>(reader: &mut R, height: usize, width: usize, bits_per_pixel: u16, color_table: Vec<Color>) -> std::io::Result<Vec<Vec<Color>>> {
        let mut pixels = Vec::new();
        let (bytes_per_line, reads_per_line) = match bits_per_pixel {
            x @ (1 | 2 | 4 | 8) => (width, ((x as usize) * width)/8),
            x @ (16 | 24 | 32) => (((x as usize) * width)/8, width),
            _ => panic!("Not implemented yet")
        };
        let num_align_bytes = if bytes_per_line % 4 == 0 { 0 } else { 4 - (bytes_per_line % 4) };

        for _ in 0..height {
            let mut line = Vec::new();
            for _ in 0..reads_per_line {
                let res = match bits_per_pixel {
                    x @ (1 | 2 | 4 | 8) => read_color_table(reader, &color_table, x),
                    16 => read_16bpp(reader),
                    24 => read_24bpp(reader),
                    32 => read_32bpp(reader),
                    _ => panic!("Not a valid bpp value")
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
    
    fn read_color_table<R: BufRead>(reader: &mut R, color_table: &Vec<Color>, bits_per_pixel: u16) -> std::io::Result<Vec<Color>> {
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
    
    fn read_16bpp<R: BufRead>(reader: &mut R) -> std::io::Result<Vec<Color>> {
        let rgb = read_u16(reader)?;
        // RGB each take 5 bit, MSB is ignored
        let mut red = ((rgb >> 10) & 0x1F) as u8;
        let mut green = ((rgb >> 5) & 0x1F) as u8;
        let mut blue = (rgb & 0x1F) as u8;
        
        // Sign extend RGB to 8bit
        let sign_extend = |color: &mut u8| {
            let sign = (*color >> 4) & 1;
            *color = (*color << 3) | 0b111*sign;
        };
        sign_extend(&mut red);
        sign_extend(&mut green);
        sign_extend(&mut blue);
        
        Ok(vec!(Color {red, green, blue}))
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