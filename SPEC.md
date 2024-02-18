keep in mind this is subject to change

# Plans

heres kinda what i want to achieve:
- use image files as the source code (involving image elements such as colour, shapes, etc. with user defined meaning to all of these elements maybe in an image key file of some kind)
- be able to transpile to c and back
- compile directly to either x86-64 or a custom instruction set
- also an interpreter and some kind of an intermediate language

# Language

Cram aims to avoid font processing as a way to interpret image data             \
i.e. cram disallows reading characters by comparing bitmaps of fonts and tiles  \
because of this, Tokens do not contain a value.

# Files

files are converted into RGB 8 bit colour depth & alpha is ignored. \
since alpha is ignored, transparent and translucent images can function as if they were opaque so invisible programs could be a fun party trick

## key

Cram projects specific syntax is defined by the user in a keys image file   \
The key file contains the symbols and colours of each token                 \
an example key image file can be found [here](examples/key.png)

the key file is a 256x256 image read in tiles (64 pixel chunks) from left to right top to bottom in a constant order which is the [key structure](https://github.com/aymey/cram/blob/main/src/processing.rs#L7)
the background colour

the background colour of the image, which is ignored (both in source and key files), is defined by the most common colour in the key file as a whole (includes what would usually be ignored colours such as grid colour)

a few quirks of key files currently:
- the grid colour is found from the very first pixel (top left corner) of the image
    - this grid colour is ignore in the key file
- non rectangular objects are tokenized from a rectangular tile (like a bounding box in video games)
    - if the amount of pixels in the tokens tile matches the amount of pixels in the keys tile, then we deem it a match
    - a keys pixels can be arranged in any way withing the bounding box
    - multiple keys with the same amount of same coloured pixels withh conflict
- key tiles are parsed imperfectly
    - every row gets increasingly offset from the top
    - the final row is not parsed at all
- a keys colour is denoted by the first non ignored (not background or grid) colour in the tile
    - the KeyData of the key is not effected because something like the amount takes into account all non ignored pixels
    - the lexing is effected by this because it counts the amount of specific coloured pixels in a tile, therefore no match will be found for multi coloured keys
### order

The order of the keys, wrapping left to right, is as follows:
1. Zero         -   The constant integer literal zero
2. Increment    -   Increments an integer by 1
3. Decrement    -   Decrements an integer by 1
4. Access       -   Declares a variable
5. Repeat       -
6. Quote        -   Interprets an integer as ASCII "printable characters"
7. Line Break   -   Denotes the end of a line

## source

The source code of Cram projects is found within image files made up of keys (see above)    \
The source files can be of any dimensions and are read from left to right in lines

line:
- each line is seperated by a line break or the x border of the image (where a linebreak is automatically inserted in lexing)
- a line has a tile with an x, y origin (top left) and a width and height
    - a lines width is defined by the distance from the origin to the closest line break
    - a lines height is defined by the greatest height of the keys that intersect a ray from the middle row of the first key to the lines width (currently, the ray does not check if the key exists, it goes off pixel colours only)

# Grammar

## syntax

### variables

variables a declared using the Access key paired with the variables symbol (like a variable name)   \
variables are referenced by their symbols

### scopes

scopes are denoted by a solid rectangle with a background colour that differs from the regular background       \
these initial definitions expect either params/return values for functions, conditions for loops, etc.          \

scopes are lexed homogeneously so side-by-side code delimited by a line break will be pushed after the scope    \
