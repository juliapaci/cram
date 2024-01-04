use image::io::Reader as ImageReader;
use image::{GenericImage, Rgb, GenericImageView, Pixel};

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Zero,
    Increment,
    Decrement,
    Access,
    Repeat,
    Quote,
    LineBreak
}

// data for the tokens
struct KeyData {
    token: Token,
    colour: Rgb<u8>,
    left_width: u8,    // width of key left of first pixel (top left)
    right_width: u8,   // width of key right of first pixel (top left)
    height: u8,
    amount: u32
}

impl KeyData {
    fn new() -> Self {
        Self {
            token: Token::Zero,
            colour: Rgb([0, 0, 0]),
            left_width: 0,
            right_width: 0,
            height: 0,
            amount: 0
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

    // language syntax
    quote: KeyData,         // for string literals
    line_break: KeyData,    // denotes a line seperation of multiple lines on the same row

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

            quote: KeyData::new(),
            line_break: KeyData::new(),

            ignore: Rgb([0, 0, 0]),
            background: Rgb([0, 0, 0]),
            grid: Rgb([0, 0, 0])
        }
    }

    // TODO: dont hardcode the size & maybe use a macro or something or use serde
    // converts the members of key to an array, excluding some members
    fn as_array(&self) -> [&KeyData; 7]  {
        [&self.zero, &self.increment, &self.decrement, &self.access, &self.repeat, &self.quote, &self.line_break]
    }

    // gets the KeyData of keys that are of the specified colour
    fn data_from_colour(&self, colour: Rgb<u8>) -> Vec<&KeyData> {
        self.as_array().iter()
            .filter(|&k| k.colour == colour)
            .copied()
            .collect::<Vec<&KeyData>>()
    }

    // TODO: make function that gives the left offset (relative to the width of the key) of the first pixel in the tile
    //       This will optimise searching tiles as it would reduce the amount of pixels that need to be search x2
    //       This will also fix a bug where multiple keys would be in one keys tile for the lexer

    // gets the background colour
    fn identify_background(&mut self, image: &image::DynamicImage) {
        let mut histogram: HashMap<Rgb<u8>, usize> = HashMap::new();
        for pixel in image.to_rgb8().pixels() {
            histogram.entry(*pixel).and_modify(|count| *count += 1).or_insert(1);
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
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();

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

    // reads the key but doesnt remove parts within it. Useful for reading hollow keys
    fn outline_key(&self, tile: &[[Rgb<u8>; 64]; 64], token: Token) -> KeyData {
        // the trimmed key
        let mut key: Vec<Vec<Rgb<u8>>> = Vec::new();

        for row in tile {
            let first = match row.iter().position(|&p| p != self.background && p != self.grid) {
                Some(i) => i,
                None => continue
            };

            // dont need to copy this but im assuming that we will need to when we identify more
            // specific attributes of each key so im leaving this here
            let last = match row.iter().copied().rev().position(|p| p != self.background && p != self.grid) {
                Some(i) => row.len() - i,
                None => continue
            };

            // trim around the key (the background outside)
            let mut left: Vec<Rgb<u8>> = row[..first]
                .iter()
                .filter(|&p| *p != self.background && *p != self.grid)
                .copied()
                .collect();
            let mut right: Vec<Rgb<u8>> = row[last..]
                .iter()
                .filter(|&p| *p != self.background && *p != self.grid)
                .copied()
                .collect();
            let mut middle = row[first..last].to_vec();

            left.append(&mut middle);
            left.append(&mut right);

            key.push(left);
        }

        let first_pixel = tile
            .iter()
            .filter(|row| {
                row
                    .iter()
                    .any(|&p| p != self.background && p != self.grid)
            })
            .flat_map(|row| row.iter())
            .position(|&p| p != self.background && p != self.grid)
            .unwrap();

        let left_most = tile
            .iter()
            .map(|row| {
                row
                    .iter()
                    .position(|&p| p != self.background && p != self.grid)
                    .unwrap_or(64)
            })
            .min().unwrap();

        let filtered: Vec<Vec<&Rgb<u8>>>= key
            .iter()
            .map(|row| {
                row.iter()
                    .filter(|&p| *p != self.background && *p != self.grid)
                    .collect::<Vec<&Rgb<u8>>>()
            })
            .collect();

        // each row is garunteed to exist with data so we can safely unwrap()
        let width = key.iter().map(Vec::len).max().unwrap() as i16;
        KeyData {
            token,
            colour: key[0][0],
            left_width: (first_pixel as i16 - left_most as i16).abs() as u8,
            right_width: (width - (first_pixel as i16 - left_most as i16)).abs() as u8,
            height: key.len() as u8,
            amount: filtered.iter().map(Vec::len).sum::<usize>() as u32
        }
    }

    // returns the KeyData of the key in a tile
    // will panic if there is nothing occupying the tile (or exclusively background and grid pixels)
    fn identify_key_data(&self, tile: &[[Rgb<u8>; 64]; 64], token: u8) -> KeyData {
        // unsafe is fine since we are hardcoding the possible values of teken
        self.outline_key(tile, unsafe {std::mem::transmute(token)})
    }

    // read each 64x64 "tile" and apply the colour inside to the key structure
    fn read_keys(&mut self, image: &image::DynamicImage) {
        self.identify_background(image);

        let tiles = self.image_to_tiles(image);
        // for (i, tile) in tiles.iter().enumerate() {
        //     Tile::from_1d(if i < 12 {256*64*(i/4)} else {0} + i*64 , 64, 64, image)
        //         .save_tile(image, format!("tile{}.png", i)).unwrap();
        // }

        // TODO: find better way of finding key grid colour like detect rangles or something
        self.grid = tiles[0][0][0];
        // TODO: better wat of doing all these actions like macro or something?
        self.zero = self.identify_key_data(&tiles[0], 0);
        self.increment = self.identify_key_data(&tiles[1], 1);
        self.decrement = self.identify_key_data(&tiles[2], 2);
        self.access = self.identify_key_data(&tiles[3], 3);
        self.repeat = self.identify_key_data(&tiles[4], 4);
        self.quote = self.identify_key_data(&tiles[5], 5);
        self.line_break = self.identify_key_data(&tiles[6], 6);
    }
}

// TODO: refactor Key parsing to use this
#[derive(Debug)]
struct Tile {
    x: usize,
    y: usize,
    width: u8,
    height: u8,
}

impl Tile {
    fn from_1d(pos: usize, width: u8, height: u8, image: &image::DynamicImage) -> Self {
        Self {
            x: pos%(image.width() as usize),
            y: pos/(image.width() as usize),
            width,
            height
        }
    }

    fn overlapping(a: &Tile, b: &Tile) -> bool {
        (a.x + a.width as usize >= b.x && b.x + b.width as usize >= a.x) &&
            (a.y + a.height as usize >= b.y && b.y + b.height as usize >= a.y)
    }

    fn save_tile(&self, source: &image::DynamicImage, name: String) -> Result<(), image::ImageError>{
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);

        for y in 0..self.height as u32 {
            for x in 0..self.width as u32 {
                if (x < img.width() && y < img.height()) &&
                    (self.x as u32 + x < source.width() && self.y as u32 + y < source.height()) {
                    img.put_pixel(x as u32, y as u32,
                                  source.get_pixel(self.x as u32 + x, self.y as u32 + y).to_rgb());
                }
            }
        }

        img.save(name)?;
        Ok(())
    }
}

struct Lexer {
    key: Key,
    tokens: Vec<Token>
}

impl Lexer {
    fn new() -> Self {
        Self {
            key: Key::new(),
            tokens: Vec::new()
        }
    }

    // returns the amount of same coloured pixels in a tile
    pub fn compute_tile(tile: &Tile, image: &image::DynamicImage, colour: Rgb<u8>) -> u32 {
        // TODO: to stop computing this everywhere maybe make a getter for it or something
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let mut amount = 0;

        let bound = (image.width()*image.height()) as isize;
        for y in 0..tile.height as usize {
            for x in 0..tile.width as isize {
                let index: isize = (tile.x as isize)+x + ((tile.y + y)*(image.width() as usize)) as isize;
                if index < 0 || index >= bound {
                    continue;
                }

                amount += (pixels[index as usize] == colour) as u32;
            }
        }

        amount
    }

    // TODO: read left to right not top to bottom
    pub fn analyse(&mut self, image: &image::DynamicImage) {
        let mut tokens: Vec<Token> = Vec::new();

        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();

        // denoting a gap between keys, used to differentiate multiple keys that have the same colour
        let gap: KeyData = KeyData {
            token: Token::Zero, // token doesnt matter
            colour: self.key.background,
            left_width: 0,
            right_width: 64,
            height: 1,
            amount: 64
        };

        let mut ignore: HashMap<Rgb<u8>, Tile> = HashMap::new();        // colours to ignore at any given point. used to not re tile keys

        let mut amount: u32 = 0;
        for (i, pixel) in pixels.iter().enumerate() {
            if *pixel == self.key.background /* || *pixel == self.key.ignore */ {
                // TODO: check for gaps
                continue;
            }

            // checking if where in an area thats already been checked
            if let Some(tile) = ignore.get(pixel) {
                if Tile::overlapping(&Tile::from_1d(i, 1, 1, image), tile) {
                    continue;
                }
            }

            // checking if the amount of pixels in the key is the same as in this tile. aiming to match lexical keys to abitrary symbols
            let keys = self.key.data_from_colour(*pixel);
            for key in keys {
                let tile = Tile::from_1d(i - key.left_width as usize, key.left_width+key.right_width, key.height, image);

                if Self::compute_tile(&tile, image, *pixel) == key.amount {
                    tokens.push(key.token);

                    amount += 1;
                    tile.save_tile(image, format!("tile{}.png", amount)).unwrap();
                }

                // dont re tile the same area
                ignore.insert(*pixel, tile);
            }
        }

        println!("{:?}", tokens);
        self.tokens = tokens;
    }
}

pub fn deserialize(key: &String, source: &String) -> Result<(), image::ImageError>{
    let key_img = ImageReader::open(key)?.with_guessed_format()?.decode()?;
    let source_img = ImageReader::open(source)?.with_guessed_format()?.decode()?;

    let mut lex = Lexer::new();
    lex.key.read_keys(&key_img);
    println!("Finished reading keys");

    lex.analyse(&source_img);
    println!("Finished tokenizing");

    Ok(())
}

// TODO: tests
