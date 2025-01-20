pub mod common {
    use std::io::{BufRead, BufWriter, Write};

    enum Endianess {
        Little,
        Big
    }

    fn slice_to_usize(bytes: &[u8], endianess: Endianess) -> usize {
        if bytes.len() > 8 {
            panic!("Slice len must be <= 8 bytes");
        }

        let mut usize = 0;
        match endianess {
            Endianess::Little => {
                for (i, &byte) in bytes.iter().enumerate() {
                    usize += (byte as usize) << i*8;
                }
            },
            Endianess::Big => {
                for (i, &byte) in bytes.iter().rev().enumerate() {
                    usize += (byte as usize) << i*8;
                }
            }
        }

        usize
    }

    pub fn slice_to_usize_le(bytes: &[u8]) -> usize {
        slice_to_usize(bytes, Endianess::Little)
    }

    pub fn slice_to_usize_be(bytes: &[u8]) -> usize {
        slice_to_usize(bytes, Endianess::Big)
    }

    pub fn read_u32<R: BufRead>(reader: &mut R) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(slice_to_usize_le(&buf) as u32)
    }
    
    pub fn read_u16<R: BufRead>(reader: &mut R) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        reader.read_exact(&mut buf)?;
        Ok(slice_to_usize_le(&buf) as u16)
    }

    pub const PAGE_SIZE: usize = 4096;
    pub fn get_larger_buffered_stdout(term_height: usize, term_width: usize) -> impl Write {
        // escape sequence for each pixel takes a few bytes, lets approximate by 16
        let size = term_height * term_width * 16;
        let aligned_size = if size % PAGE_SIZE == 0 { size } else { ((size / PAGE_SIZE) + 1) * PAGE_SIZE };
        
        BufWriter::with_capacity(aligned_size, std::io::stdout().lock())
    }
}