pub mod gif {
    use std::io::{Error, Write};
    use std::fs::File;
    use std::path::Path;

    use termsize::Size;
    use gif::{Decoder, DecodeOptions};

    use crate::ansi::ansi::{self, Erase, Color};
    use crate::common::common::get_larger_buffered_stdout;

    pub struct Gif {
        decoder: Decoder<File>
    }

    impl Gif {
        pub fn new(path: &Path) -> std::io::Result<Self> {
            let mut options = DecodeOptions::new();
            options.set_color_output(gif::ColorOutput::RGBA);

            let f = File::open(path)?;
            let decoder = match options.read_info(f) {
                Ok(decoder) => decoder,
                Err(err) => return Err(Error::other(format!("GIF Decoder Error: {err}")))
            };

            Ok(Gif {decoder})
        }
        
        pub fn print(&mut self, term_size: &Size) -> std::io::Result<()> {
            let term_height = term_size.rows as usize;
            let term_width = term_size.cols as usize;
            let mut writer = get_larger_buffered_stdout(term_height, term_width);

            let mut prev = false;
            while let Some(frame) = self.decoder.read_next_frame().unwrap() {
                if frame.interlaced {
                    panic!("Frame interlaced");
                }

                print_frame(term_size, &mut writer, frame, prev)?;
                prev = true;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            Ok(())
        }
    }
    
    
    fn print_frame<W: Write>(term_size: &Size, writer: &mut W, frame: &gif::Frame, has_prev: bool) -> std::io::Result<()> {
        let term_height = term_size.rows as usize;
        let term_width = term_size.cols as usize;
        if !has_prev {
            ansi::erase(Erase::SCREEN, writer)?;
        }
        ansi::reset_cursor(writer)?;

        let y_step: f64 = f64::max((frame.height as f64) / (term_height as f64), 1.0);
        let x_step: f64 = f64::max((frame.width as f64) / (term_width as f64), 1.0);
        let height = std::cmp::min(frame.height as usize, term_height);
        let width = std::cmp::min(frame.width as usize, term_width);

        let mut fy: f64 = 0.0;
        for _ in 0..height {
            let y = fy.floor() as usize;
            let mut fx: f64 = 0.0;
            for _ in 0..width {
                let x = fx.floor() as usize;
                fx += x_step;

                let indx = y * height + x;
                let rgba: [u8; 4] = frame.buffer[indx*4..(indx+1)*4].try_into().unwrap();
                let is_transparent_pixel = rgba[3] == 0;
                let color = Color::from(u32::from_be_bytes(rgba) >> 8);

                if is_transparent_pixel && has_prev {
                    ansi::cursor_forward(1, writer)?;
                    continue;
                }

                color.print(writer)?;
            }
            fy += y_step;
            ansi::next_line(writer)?;
        }
        writer.flush()?;

        Ok(())
    }
}