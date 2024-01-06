use image::io::Reader as ImageReader;
use image::{GenericImage, Rgb, GenericImageView, Pixel};

use std::collections::HashMap;

// TODO: refactor Key parsing to use this
#[derive(Debug)]
struct Tile {
    x: usize,
    y: usize,
    width: u32,
    height: u32,
}

impl Tile {
    fn from_1d(pos: usize, width: u32, height: u32, image: &image::DynamicImage) -> Self {
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
            token: Token::LineBreak,
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

    // returns the KeyData of a token
    fn data_from_key(&self, key: Token) -> &KeyData {
        // unsafe is fine since every token has an index in the array since its hardcoded (see as_array())
        self
            .as_array()
            [unsafe {std::mem::transmute::<Token, u8>(key)} as usize]
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

struct Lexer {
    key: Key,
    tokens: Vec<Token>
}

impl Lexer {
    fn new() -> Self {
        Self {
            key: Key::new(),
            tokens: Vec::new()  // Token buffer
        }
    }

    // returns the amount of same coloured pixels in a tile
    pub fn compute_tile(tile: &Tile, colour: Rgb<u8>, image: &image::DynamicImage) -> u32 {
        // TODO: to stop computing this everywhere maybe make a getter for it or something
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let mut amount = 0;

        let bound = (image.width()*image.height()) as isize;
        for y in 0..tile.height as usize {
            for x in 0..tile.width as isize {
                let index: isize = (tile.x as isize)+x + ((tile.y + y)*(image.width() as usize)) as isize;
                if index < 0 || index >= bound {
                    // continue;
                    return 0;
                }

                amount += (pixels[index as usize] == colour) as u32;
            }
        }

        amount
    }

    // returns the first key from a 1d index
    fn consume_first(&self, begin: usize, image: &image::DynamicImage) -> &KeyData {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        for i in begin..image.width() as usize*image.height() as usize {
            if pixels[i] == self.key.background {
                continue;
            }

            let keys = self.key.data_from_colour(pixels[i]);
            for key in keys {
                let tile = Tile::from_1d(
                    i.max(key.left_width as usize) - key.left_width as usize,
                    (key.left_width+key.right_width) as u32,
                    key.height as u32,
                    image
                );

                // if the tile matches a key
                if Self::compute_tile(&tile, pixels[i], image) == key.amount {
                    return self.key.data_from_key(key.token);
                }
            }
        }

        // if theres no first key
        // very unlikely but could happen
        &self.key.line_break
    }

    // return the height of the line
    // its just the tallest key that intersects a ray from the first keys middle row
    // NOTE: does not take into account BreakLines
    fn line_height(&self, begin: usize, image: &image::DynamicImage) -> u8 {
        // unwrapping is fine since there is always atleast one element when this function is called
        let first = self.consume_first(begin, image);
        let mut ignore: HashMap<Rgb<u8>, _> = HashMap::new();
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let mut max_height: u8 = first.height;

        // y index of middle row of key
        // beginning y + half key height
        let middle_row = (begin as u32/image.width() + (first.height/2) as u32)  * image.width();

        for i in begin%image.width() as usize..image.width() as usize {
            // TODO: see if we should check if the key exists instead of just relying on one pixel
            //       pros: more accurate line height + possibly faster tokenization
            //       cons: slower + more accurate tokenization

            let colour = pixels[i + middle_row as usize];
            if colour == self.key.background {
                continue
            }

            if let Some(_) = ignore.get(&colour) {
                continue;
            }
            ignore.insert(colour, true);

            max_height = self.key
                .data_from_colour(colour)
                .iter()
                .map(|&k| k.height)
                .max()
                .unwrap()
                .max(max_height);
        }

        max_height
    }

    // tokenizes a line of keys
    // returns the tokens and size of line
    fn analyse_line(&self, begin: usize, image: &image::DynamicImage) -> (Vec<Token>, Tile) {
        let mut line: Vec<Token> = Vec::new();                                      // token buffer
        let mut size = Tile::from_1d(begin, 0, self.line_height(begin, image) as u32, image);  // size of line
        let mut ignore: HashMap<Rgb<u8>, Tile> = HashMap::new();
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();

        'img: for i in begin..(begin/image.width() as usize + size.height as usize) * image.width() as usize + image.width() as usize {
            if pixels[i] == self.key.background /* || *pixel == self.key.ignore */ {
                continue;
            }

            // checking if where in an area thats already been checked
            if let Some(tile) = ignore.get(&pixels[i]) {
                // TODO: fix ignoring
                if Tile::overlapping(&Tile::from_1d(i, 1, 1, image), tile) {
                    continue;
                }
            }

            // checking if the amount of pixels in the key is the same as in this tile. aiming to match lexical keys to abitrary symbols
            let keys = self.key.data_from_colour(pixels[i]);
            for key in keys {
                let tile = Tile::from_1d(
                    i.max(key.left_width as usize) - key.left_width as usize,
                    (key.left_width+key.right_width) as u32,
                    key.height as u32,
                    image
                );

                // if the tile matches a key
                if Self::compute_tile(&tile, pixels[i], image) == key.amount {
                    line.push(key.token);

                    // debug stuff
                    // tile.save_tile(image, format!("tile{}.png", i-begin)).unwrap();
                    // println!("found {:?}", key.token);

                    if key.token == Token::LineBreak {
                        size.width = (i%image.width() as usize) as u32;
                        break 'img;
                    }
                }

                // marks this area as already checked
                ignore.insert(pixels[i], tile);
            }
        }

        // inserting a line break if there wasnt one there
        // unwrapping is fine since there will always be atleast 1 token
        if *line.last().unwrap() != Token::LineBreak {
            line.push(Token::LineBreak);
        }

        (line, size)
    }

    pub fn analyse(&mut self, image: &image::DynamicImage) {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();

        let mut i = 0;
        while i < pixels.len() {
            if pixels[i] == self.key.background /* || *pixel == self.key.ignore */ {
                i += 1;
                continue;
            }

            let mut line = self.analyse_line(i, image);
            i += line.1.width as usize + line.1.height as usize *image.width() as usize;
            self.tokens.append(&mut line.0);

            i += 1;
        }
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
    println!("{:?}", lex.tokens);

    Ok(())
}

// TODO: tests
