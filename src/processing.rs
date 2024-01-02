use image::io::Reader as ImageReader;
use image::{GenericImageView, Rgb};

use std::collections::HashMap;

// what the colours mean
struct KeyData {
    colour: Rgb<u8>,
    size: u16
}

impl KeyData {
    fn new() -> Self {
        Self {
            colour: Rgb([0, 0, 0]),
            size: 0
        }
    }
}

struct Key {
    // for turing completeness
    zero: KeyData,          // the constant `0`
    increment: KeyData,     // increment a value
    decrement: KeyData,     // decrement a value
    access: KeyData,        // access a memory address
    repeat: KeyData,        // jump based on a condition

    // language specific syntax
    string: KeyData,        // for string literals
    line_break: KeyData,

    // not a token
    ignore: Rgb<u8>,        // a colour to ignore
    background: Rgb<u8>,    // background colour of the image
    grid: Rgb<u8>           // grid colour for the key file
}

impl Key {
    fn new() -> Self {
        Self {
            zero: KeyData::new(),
            increment: KeyData::new(),
            decrement: KeyData::new(),
            access: KeyData::new(),
            repeat: KeyData::new(),

            string: KeyData::new(),
            line_break: KeyData::new(),

            ignore: Rgb([0, 0, 0]),
            background: Rgb([0, 0, 0]),
            grid: Rgb([0, 0, 0])
        }
    }

    // TODO: dont hardcode the size & maybe use a macro or something or use serde
    // converts the members of key to an array, excluding some members
    fn as_array(&self) -> [&KeyData; 7]  {
        [&self.zero, &self.increment, &self.decrement, &self.access, &self.repeat, &self.string, &self.line_break]
    }

    // gets the size of keys that are of the specified colour
    fn size_from_colour(&self, colour: Rgb<u8>) -> Vec<u16> {
        self.as_array().iter()
            .filter(|&k| k.colour == colour)
            .map(|k| k.size)
            .collect::<Vec<u16>>()
    }

    // gets the rectangle (width, height) of the keys of the specified colour
    // fn rect_from_colour(&self, colour:Rgb<u8>) -> Vec<[u16; 2]> {
    //     self.as_array().iter()
    //         .filter(|&k| )
    // }

    // gets the background colour
    fn identify_background(&mut self, image: &image::DynamicImage) {
        let mut histogram: HashMap<Rgb<u8>, usize> = HashMap::new();
        for pixel in image.to_rgb8().pixels() {
            let counter = histogram.entry(*pixel).or_insert(0);
            *counter += 1;
        }

        let background = histogram
            .iter()
            .max_by_key(|(_, &count)| count)
            .unwrap_or((&Rgb([0, 0, 0]), &0));

        self.background = *background.0;
    }

    // TODO: make it more flexible so the key file isnt restricted to a certain resolution
    // splits an image into 64x64 chunks
    fn image_to_tiles(&mut self, image: &image::DynamicImage) -> [[[Rgb<u8>; 64]; 64]; 16]{
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().map(|&p| p).collect();

        let mut tiles: [[[Rgb<u8>; 64]; 64]; 16] = [[[Rgb([0, 0, 0]); 64]; 64]; 16];
        for tile in 0..16 {
            for y in 0..64 {
                for x in 0..64 {
                    // TODO: fix slight errors where each row gets increasingly offset some pixels. (luckily doesnt effet key parsing)
                    // row of tiles offset (4 tiles) + tile offset + y tile offset + x tile offset
                    tiles[tile][y][x] = pixels[if tile < 12 {256*64*(tile/4)} else {0} + tile*64 + 256*y + x];
                }
            }
        }

        tiles
    }

    fn save_tiles(&self, tiles: &[[[Rgb<u8>; 64]; 64]; 16]) -> Result<(), image::ImageError>{
        for (i, tile) in tiles.iter().enumerate() {
            let mut img = image::RgbImage::new(64, 64);
            for (y, column) in tile.iter().enumerate() {
                for (x, row) in column.iter().enumerate() {
                    img.put_pixel(x as u32, y as u32, *row);
                }
            }
            img.save(format!("tile{}.png", i))?;
        }

        Ok(())
    }

    // returns the colour of the key
    // will panic if there is nothing occupying the tile (excluding background and grid)
    fn identify_key_data(&self, tile: &[[Rgb<u8>; 64]; 64]) -> KeyData {
        let pixels: Vec<&Rgb<u8>> = tile
            .iter()
            .flat_map(|row| {
                row.iter()
                    .filter(|&p| *p != self.background && *p != self.grid)
            })
            .collect();

        KeyData {
            colour: *pixels[0],
            size: pixels.len() as u16       // tile cant be more than u16 so converting from usize is safe
        }
    }

    // read each 64x64 "tile" and apply the colour inside to the key structure
    fn read_keys(&mut self, image: &image::DynamicImage) {
        self.identify_background(image);

        let tiles = self.image_to_tiles(image);
        self.save_tiles(&tiles).unwrap();
        // TODO: find better way of finding key grid colour
        self.grid = tiles[0][0][0];

        // TODO: better wat of doing all these actions like macro or something?
        self.zero = self.identify_key_data(&tiles[0]);
        self.increment = self.identify_key_data(&tiles[1]);
        self.decrement = self.identify_key_data(&tiles[2]);
        self.access = self.identify_key_data(&tiles[3]);
        self.repeat = self.identify_key_data(&tiles[4]);
        self.string = self.identify_key_data(&tiles[5]);
        self.line_break = self.identify_key_data(&tiles[6]);
    }
}

struct Lexer {
    key: Key,
    tokens: Vec<u8>
}

// TODO: refactor Key parsing to use this
// TODO: maybe refactor to 1d for the 1d pixel array from image
struct Tile {
    x: usize,
    y: usize,
    width: u16,
    height: u16,
}

impl Tile {
    fn from_1d(pos: usize, width: u16, height: u16, image: &image::DynamicImage) -> Self {
        Self {
            x: pos/(image.width() as usize),
            y: pos/(image.width() as usize),
            width,
            height
        }
    }
}

impl Lexer {
    fn new() -> Self {
        Self {
            key: Key::new(),
            tokens: Vec::new()
        }
    }

    // returns the amount of same coloured pixels in a tile
    pub fn compute_tile(tile: Tile, image: &image::DynamicImage, colour) -> u32 {
        // TODO: to stop computing this everywhere maybe make a getter for it or something
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().map(|&p| p).collect();
        let mut amount = 0;

        // i hate that we have to do this in rust. its saefe not to but the compiler still complains :(
        let left = -(tile.width as isize);
        for y in 0..tile.height as usize {
            for x in left..tile.width as isize {
                let index: isize = (tile.x as isize)+x + ((tile.y + y)*(image.width() as usize)) as isize;
                if index < 0 {
                    continue;
                }

                amount += (pixels[index as usize] == colour) as u32
            }
        }

        amount
    }

    pub fn lex(&self, image: &image::DynamicImage) {
        let tokens: Vec<u8> = Vec::new();

        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().map(|&p| p).collect();

        // denoting a gap between keys, used to differentiate multiple keys that have the same colour
        let gap: KeyData = KeyData {
            colour: self.key.background,
            size: 64
        };

        // will keep track of "key" tokens that have been encountered, including their size and colour
        let mut token_buffer: HashMap<Rgb<u8>, usize> = HashMap::new();
        for (i, pixel) in pixels.iter().enumerate() {
            let size = token_buffer.entry(*pixel).or_insert(0);
            *size += 1;

            if *pixel == self.key.background || *pixel == self.key.ignore {
                continue;
            }

            // if(compute_tile(Tile::from_1d(i, self.key.size_from_colour())))
        }
    }
}
pub fn deserialize(key: &String, source: &String) -> Result<(), image::ImageError>{
    let key_img = ImageReader::open(key)?.with_guessed_format()?.decode()?;
    let source_img = ImageReader::open(source)?.with_guessed_format()?.decode()?;

    let mut lex = Lexer::new();
    lex.key.read_keys(&key_img);

    lex.lex(&source_img);

    Ok(())
}
