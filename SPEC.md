# Plans
heres kinda what i want to achieve:
- use image files as the source code (involving image elements such as colour, shapes, etc. with user defined meaning to all of these elements maybe in an image key file of some kind)
- be able to transpile to c and back
- compile from c to either x86-64 or a custom instruction set
- also an interpreter and some kind of an intermediate language

# Files
Cram program files depend on a ruleset defined by the user in a keys image file
The key file contains the symbols and colours of each token
an example key image file can be found [here](examples/key.png)

the key file is a 256x256 image read in tiles (64 pixel chunks) from left to right top to bottom in a constant order which is the [key structure](https://github.com/aymey/cram/blob/main/src/processing.rs#L7)
