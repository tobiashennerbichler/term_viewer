# term_viewer

## Information

Application written in Rust that allows the drawing of Windows Bitmap images directly in the Terminal. The resolution of the displayed image is dependent on the size of the terminal (see examples below). Implements a Windows Bitmap file parser with support for all possible bpp values (1, 4, 8, 16, 24, 32) but does not support compressed images. Also supports displaying multiple Windows Bitmap files that are in the same folder in a row to implement some basic "animations".

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
