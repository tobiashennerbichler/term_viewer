# term_viewer

Application written in Rust that allows displaying of Bitmap images directly in the Terminal. Resolution of the image is dependent on the terminal size.

## Running
The application can be run using "cargo run -- file.bmp"

## TODOs:
- Currently only consists of a basic Bitmap parser (24bbp) but this will be extended to the remaining bpp values.
- Experimenting with different down-scale operations (e.g. taking average of adjacent pixels)
