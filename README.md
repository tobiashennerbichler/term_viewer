# term_viewer

## Information

Application written in Rust that allows displaying of Windows Bitmap images directly in the Terminal. Provides support for all possible bpp values (1, 4, 8, 16, 24, 32) but does not support compressed images. Also supports displaying multiple Windows Bitmap files that are in the same folder in a row to implement some basic "animations". Resolution of the image is dependent on the terminal size.

### TODOS:
- gif support
- The interactive branch implements an interactive window to select which file to display in the current folder. Still WIP.

## Running

The application can be run using "cargo run -- file.bmp"

## Example

An example of displaying the file *test_images/tree.bmp* with two different terminal sizes:

Terminal 133x36:
![image](markdown_images/tree_low.png)
Terminal 1264x269:
![image](markdown_images/tree_high.png)
