pub mod bitmap {
    use std::{fs::File, io::Read};
    use std::io::Error;

    const BITMAP_FILE_HEADER_LEN: usize = 14;
    //const BITMAP_DIB_HEADER_LEN: usize = 10;

    #[derive(Copy, Clone)]
    struct Color {
        red: u8,
        green: u8,
        blue: u8
    }

    impl Into<String> for Color {
        fn into(self) -> String {
            format!("{};{};{}", self.red, self.green, self.blue)
        }
    }
    
    impl Color {
        pub fn print(self) {    
            let s: String = self.into();
            print!("\x1b[38;2;{}m█\x1b[m", s);
        }
    }

    pub struct Bitmap {
        width: usize,
        height: usize,
        pixels: Vec<Color>
    }
    
    impl Bitmap {
        pub fn new(filename: &str) -> std::io::Result<Self> {
            let mut file = File::open(filename)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;

            Ok(Bitmap {width: 0, height: 0, pixels: Vec::new()})
        }
    }

    enum Endianess {
        Little,
        Big
    }

    fn slice_to_usize(bytes: &[u8], endianess: Endianess) -> std::io::Result<usize> {
        if bytes.len() > 8 {
            return Err(Error::other("Bytes length must be <= 8"));
        }

        let mut usize = 0;
        match endianess {
            Endianess::Little => {
                for (i, &byte) in bytes.iter().enumerate() {
                    println!("byte: 0x{byte:0x} at {i}");
                    usize += (byte as usize) << i*8;
                }
            },
            Endianess::Big => {
                for (i, &byte) in bytes.iter().rev().enumerate() {
                    usize += (byte as usize) << i*8;
                }
            }
        }

        Ok(usize)
    }

    pub fn parse_bitmap_header(bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() < BITMAP_FILE_HEADER_LEN {
            return Err(Error::other("File does not contain BITMAP header"));
        }
        
        if b"BM" != &bytes[..2] {
            return Err(Error::other("File does not contain BITMAP magic values"));
        }
        
        let len = slice_to_usize(&bytes[2..6], Endianess::Little).unwrap();
        if len != bytes.len() {
            println!("read len: {len}, file len: {}", bytes.len());
            return Err(Error::other("File indicates wrong file size"));
        }
        
        let offset = slice_to_usize(&bytes[10..14], Endianess::Little).unwrap();
        Ok(offset)
    }
}