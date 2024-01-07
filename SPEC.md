keep in mind this is subject to change

# Plans
heres kinda what i want to achieve:
- use image files as the source code (involving image elements such as colour, shapes, etc. with user defined meaning to all of these elements maybe in an image key file of some kind)
- be able to transpile to c and back
- compile directly to either x86-64 or a custom instruction set
- also an interpreter and some kind of an intermediate language

# Files
files are converted into RGB 8 bit colour depth & alpha is ignored

## key
Crams project specific syntax is defined by the user in a keys image file   \
The key file contains the symbols and colours of each token                 \
an example key image file can be found [here](examples/key.png)

the key file is a 256x256 image read in tiles (64 pixel chunks) from left to right top to bottom in a constant order which is the [key structure](https://github.com/aymey/cram/blob/main/src/processing.rs#L7)

a few quirks of key files currently:
- the grid colour is found from the very first pixel (top left corner) of the image
    - this grid colour is ignore in the key file
- non rectangular objects are tokenized from a rectangular tile (like a bounding box in video games)
    - if the amount of pixels in the tokens tile matches the amount of pixels in the keys tile, then we deem it a match
    - due to this, a keys pixels can be arranged in any way withing the bounding box
- key tiles are parsed imperfectly
    - every row gets more and more offset from the top
    - the final row is not parsed at all

## source
The source code of Cram projects is found within image files made up of keys (see above)    \
The source files can be of any dimensions and are read from left to right in lines

line:
- each line is seperated by a line break or the x border of the image (where a linebreak is automatically inserted in lexing)
- a line has a tile with an x, y origin (top left) and a width and a height
    - a lines width is defined by the distance from the origin and the closest (x wise) line break
    - a lines height is defined by the greatest height of the keys that intersect a ray from the middle row of the first key in a line (currently, the ray does not check if the key exists, it goes off pixel colours only)
