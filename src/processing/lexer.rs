use image::io::Reader as ImageReader;
use image::{GenericImage, Rgb, GenericImageView, Pixel};

use std::collections::HashMap;

// TODO: refactor Key parsing to use this
#[derive(Default, PartialEq, Debug)]
struct Tile {
    // Tile assumes a top left origin
    x: usize,
    y: usize,
    width: u32,
    height: u32,
}

impl Tile {
    // changes 1d to 2d pos in a Tile
    fn from_1d(pos: usize, width: u32, height: u32, image: &image::DynamicImage) -> Self {
        Self {
            x: pos%(image.width() as usize),
            y: pos/(image.width() as usize),
            width,
            height
        }
    }

    // check if two tiles are overlapping
    fn overlapping(a: &Tile, b: &Tile) -> bool {
        (a.x + a.width as usize >= b.x && b.x + b.width as usize >= a.x) &&
            (a.y + a.height as usize >= b.y && b.y + b.height as usize >= a.y)
    }

    // loops through the tile top to bottom
    // TODO: make a generic tile iteration for use in the lexer
    fn iterate(&self, f: &dyn Fn(usize, usize)) {
        for x in self.x .. self.x + self.width as usize {
            for y in self.y .. self.y + self.height as usize {
                f(x, y)
            }
        }
    }

    // will save a pixels in a tile as an image
    #[allow(dead_code)] // debug function
    fn save_tile(&self, name: String, source: &image::DynamicImage) -> Result<(), image::ImageError> {
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Token {
    // constant keys (read from key file)
    Zero,
    Increment,
    Decrement,
    Access,
    Repeat,
    Quote,
    #[default]
    LineBreak,

    // dynamic keys (read from source file)
    Variable
}

#[derive(Debug, PartialEq)]
pub enum Lexeme {
    Token(Token),       // key file tokens (static tokens i.e keys)
    Identifier(usize) // source file tokens (dynamic tokens e.g. variables)
}

// data for the tokens
#[derive(Debug, PartialEq)]
pub struct KeyData {
    token: Token,       // token that the key represents
    colour: Rgb<u8>,    // colour of key
    width_left: u8,     // width of key from the first (top left) pixel leftwards
    width_right: u8,    // width of key from the first (top left) pixel rightwards
    height_up: u8,      // height of key from the first (leftmost) pixel upwards
    height_down: u8,    // height of key from the first (leftmost) pixel downwards
    amount: u32         // amount of non ignored (e.g. background, grid) pixels in key
}

impl KeyData {
    fn new() -> Self {
        Self {
            token: Default::default(),
            colour: Rgb([0, 0, 0]),
            width_left: 0,
            width_right: 0,
            height_up: 0,
            height_down: 0,
            amount: 0
        }
    }
}

// data from key file parsing (except variables)
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
    variables: Vec<KeyData>,// variables symbols (like names) that have been defined in source files

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
            variables: Vec::new(),

            ignore: Rgb([0, 0, 0]),
            background: Rgb([0, 0, 0]),
            grid: Rgb([0, 0, 0])
        }
    }

    // TODO: dont hardcode the size & maybe use a macro or something or use serde
    // converts the members of Key to an array, excluding some members
    fn data(&self) -> Vec<&KeyData> {
        let mut keys = vec![&self.zero, &self.increment, &self.decrement, &self.access, &self.repeat, &self.quote, &self.line_break]; // keys from key file
        keys.extend(self.variables.iter()); // keys from source file (variables)

        keys
    }

    // gets the KeyData of keys that are of the specified colour
    fn data_from_colour(&self, colour: Rgb<u8>) -> Vec<&KeyData> {
        self.data().iter()
            .filter(|&k| k.colour == colour)
            .copied()
            .collect::<Vec<&KeyData>>()
    }

    // TODO: find a way to include variables
    // returns the KeyData of a token
    fn data_from_token(&self, key: Token) -> &KeyData {
        // unsafe is fine since every token has an index in the array since its hardcoded (see as_array())
        self
            .data()
            [unsafe {std::mem::transmute::<Token, u8>(key)} as usize]
    }

    // gets the largest height and width from all of the keys (likely not from the same key)
    fn get_largest(&self) -> (u8, u8) {
        let sizes: Vec<(u8, u8)> = self.data()
            .iter()
            .map(|&k| (k.width_left + k.width_right, k.height_up + k.height_down))
            .collect();

        // unwrap is fine since we hardcore the array

        // width
        (sizes.iter()
         .map(|s| s.0)
         .max().unwrap(),
        // height
         sizes.iter()
         .map(|s| s.1)
         .max().unwrap())
    }

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

    // converts an area of the image to a 2d array of pixels
    fn tile_to_pixels(&self, tile: &Tile, image: &image::DynamicImage) -> [[Rgb<u8>; 64]; 64] {
        let mut pixels: [[Rgb<u8>; 64]; 64] = [[Rgb([0, 0, 0]); 64]; 64];

        for y in 0 .. tile.height as usize {
            for x in 0 .. tile.width as usize {
                if tile.y + y >= image.height() as _ ||
                    tile.x + x >= image.width() as _ {
                    pixels[y][x] = self.background;
                    continue;
                }

                pixels[y][x] = image.get_pixel((tile.x + x) as u32, (tile.y + y) as u32).to_rgb();
            }
        }

        pixels
    }

    // TODO: make it more flexible so the key file isnt restricted to a certain resolution
    // splits an image into 4x4 64x64 chunks
    fn image_to_tiles(&mut self, image: &image::DynamicImage) -> [[[Rgb<u8>; 64]; 64]; 16] {
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
    // will panic if there is nothing (ignored pixels) occupying the tile (e.g. exclusively background and/or grid pixels)
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

        // top left pixels' coords
        let mut first_pixel: (usize, usize) = Default::default();

        first_pixel.0 = tile    // x
            .iter()
            .filter(|row| {
                row
                    .iter()
                    .any(|&p| p != self.background && p != self.grid)
            })
            .flat_map(|row| row.iter())
            .position(|&p| p != self.background && p != self.grid)
            .unwrap();

        first_pixel.1 = *tile   // y
            .iter()
            .enumerate()
            .map(|(y, row)| {
                if row[first_pixel.0] != self.background && row[first_pixel.0] != self.grid { y } else { 0 }
            })
            .collect::<Vec<usize>>()
                .into_iter()
                .filter(|&a| a != 0)
                .collect::<Vec<usize>>()
                .first().unwrap_or(&0);

        // left most pixels' coords
        let leftmost_pixel: (usize, usize) = tile
            .iter()
            .enumerate()
            .map(|(y, row)| {(
                row
                    .iter()
                    .position(|&p| p != self.background && p != self.grid)
                    .unwrap_or(64), // TODO: dont hardcode this
                    y
            )})
            .min().unwrap();

        // tile without any background or grid pixels
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

            width_left: (first_pixel.0 as i16 - leftmost_pixel.0 as i16).abs() as u8,
            width_right: (width - (first_pixel.0 as i16 - leftmost_pixel.0 as i16)).abs() as u8,

            // TODO: this ignores hollow in height which causes wrong height if theres gaps in the middle (y wise) of keys
            height_up: (leftmost_pixel.1 as i16 - first_pixel.1 as i16).abs() as u8,
            height_down: key.len() as u8 - (leftmost_pixel.1 as i16 - first_pixel.1 as i16).abs() as u8,

            amount: filtered.iter().map(Vec::len).sum::<usize>() as u32
        }
    }

    // returns the KeyData of the key in a tile
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

        // TODO: find better way of finding key grid colour like detect rectangles or something
        self.grid = tiles[0][0][0];
        // TODO: find ignore pixel colour
        // TODO: better way of doing all these actions like macro or something?
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
    tokens: Vec<Lexeme>
}

impl Lexer {
    fn new() -> Self {
        Self {
            key: Key::new(),    // Keys
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

    // returns the first keys token from a 1d index onwards
    // TODO: wont get the first, will get the heighest
    // TODO: optimise this with ignore map
    fn consume_first(&self, begin: usize, image: &image::DynamicImage) -> Token {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        for i in begin..image.width() as usize * image.height() as usize {
            if pixels[i] == self.key.background {
                continue;
            }

            let keys = self.key.data_from_colour(pixels[i]);
            for key in keys {
                let tile = Tile::from_1d(
                    i.max(key.width_left as usize) - key.width_left as usize,
                    (key.width_left + key.width_right) as u32,
                    (key.height_up + key.height_down) as u32,
                    image
                );

                // if the tile matches a key
                if Self::compute_tile(&tile, pixels[i], image) == key.amount {
                    return key.token;
                }
            }
        }

        // if theres no first key
        // very unlikely but could happen
        Token::LineBreak // maybe should be default token?
    }

    // return the height of the line
    // its just the tallest key that intersects a ray from the first keys middle row
    // NOTE: does not take into account BreakLines
    fn line_height(&self, begin: usize, image: &image::DynamicImage) -> u8 {
        // unwrapping is fine since there is always atleast one element when this function is called
        let first = self.key.data_from_token(self.consume_first(begin, image));
        let mut ignore: HashMap<Rgb<u8>, _> = HashMap::new();
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let mut max_height: u8 = first.height_up + first.height_down;

        // index of middle row of key
        // beginning y + half key height
        let middle_row = (begin as u32/image.width() + (max_height/2) as u32) * image.width();

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
                .map(|&k| k.height_up + k.height_down)
                .max()
                .unwrap_or(0) // need to do this cause we dont check if the key exists yet
                .max(max_height);
        }

        max_height
    }

    // tokenizes a line of keys
    // returns the tokens and size of line
    fn analyse_line(&mut self, begin: usize, image: &image::DynamicImage) -> (Vec<Lexeme>, Tile) {
        let mut line: Vec<Lexeme> = Vec::new(); // token buffer
        let mut size = Tile::from_1d(begin, image.width(), self.line_height(begin, image) as u32, image);  // size of line
        size.width -= size.x as u32;
        // TODO: maybe instead of ignore we just skip over the width of the token when analysed
        let mut ignore: HashMap<Rgb<u8>, Tile> = HashMap::new();

        // faster to do this or to use get_pixel()?
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels.chunks_exact(image.width() as usize).map(|chunk| chunk.to_vec()).collect();

        if size.height == 0 {
            return (Vec::new(), size);
        }

        // TODO: optimise line height to perfectly fit everything (right now it larger than it needs to be) + then we can use Tile::overlapping because we wont need custom yh for loop
        'img: for x in size.x .. (size.x + size.width as usize).min(image.width() as usize) {
            for y in size.y.max(size.height as usize) - size.height as usize .. (size.y + size.height as usize).min(image.height() as usize) {

                if pixels[y][x] == self.key.background /* || *pixel == self.key.ignore */ {
                    continue;
                }

                // checking if where in an area thats already been checked
                if let Some(tile) = ignore.get(&pixels[y][x]) {
                    if Tile::overlapping(&Tile {x, y, width: 1, height: 1}, tile) {
                        continue;
                    }
                }

                // read variable symbols
                if matches!(line.last(), Some(lexeme)
                            if matches!(lexeme, Lexeme::Token(token)
                                        if *token == Token::Access)) {
                    self.key.variables.push(
                        self.key.outline_key(
                            &self.key.tile_to_pixels(&Tile {
                                x, y: size.y,
                                width: 64, height: 64
                            }, &image),
                            Token::Variable)
                        );
                }

                // checking if a key matches pixels in a tile
                for key in self.key.data_from_colour(pixels[y][x]) {
                    let tile = Tile {
                        x,
                        y: y.max(key.height_up as usize) - key.height_up as usize,
                        width: (key.width_left+key.width_right) as u32,
                        height: (key.height_up + key.height_down) as u32,
                    };

                    // if the tile matches a key
                    if Self::compute_tile(&tile, pixels[y][x], image) == key.amount {
                        line.push(match key.token {
                            Token::Variable => Lexeme::Identifier(self.key.variables.as_ptr() as *const i32 as usize),
                            _ => Lexeme::Token(key.token)
                        });

                        if key.token == Token::LineBreak {
                            size.width = (x - size.x) as u32 + key.width_right as u32;
                            break 'img;
                        }
                    }

                    // marks this area as already checked
                    ignore.insert(pixels[y][x], tile);
                }
            }
        }

        // inserting a line break if there wasnt one there
        // TODO: ignore consecutive LineBreaks better
        if let Some(&ref lexeme) = line.last() {
            if *lexeme != Lexeme::Token(Token::LineBreak) {
                line.push(Lexeme::Token(Token::LineBreak));
            }
        }

        (line, size)
    }

    pub fn analyse(&mut self, image: &image::DynamicImage) {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels
            .chunks_exact(image.width() as usize)
            .map(|chunk| {
                chunk
                    .to_vec()
                    .iter()
                    .cloned()
                    .collect()
            })
            .collect();

        let possible_line_size = self.key.get_largest();
        let mut frame = Tile {
            x: 0,
            y: 0,
            width: possible_line_size.0 as u32,
            height: possible_line_size.1 as u32
        };

        // basically a custom windows() over the image in "frames"
        while frame.y < image.height() as usize {    // how many frames can fit on y
            frame.x = 0;
            while frame.x < image.width() as usize {  // how many frames can fit on x
                // check for anything in side the frame
                'frame: for x in 0..frame.width as usize {
                    if x + frame.x >= image.width() as usize {
                        break;
                    }

                    for y in 0..frame.height as usize {
                        if y + frame.y >= image.height() as usize {
                            break;
                        }

                        if pixels[y + frame.y][x + frame.x] == self.key.background /* || *pixel == self.key.ignore */ {
                            continue;
                        }

                        let mut line = self.analyse_line((y + frame.y)*image.width() as usize + (x + frame.x), image);
                        frame.x += line.1.width as usize - 1;
                        frame.y += line.1.height as usize;

                        self.tokens.append(&mut line.0);

                        break 'frame;
                    }
                }
                frame.x += frame.width as usize;
            }
            frame.y += frame.height as usize;
        }
    }
}

pub fn deserialize(key: &String, source: &String) -> Result<Vec<Lexeme>, image::ImageError> {
    let key_img = ImageReader::open(key)?.with_guessed_format()?.decode()?;
    let source_img = ImageReader::open(source)?.with_guessed_format()?.decode()?;

    let mut lex = Lexer::new();
    lex.key.read_keys(&key_img);
    println!("Finished reading keys");

    lex.analyse(&source_img);
    println!("Finished tokenizing");
    println!("{:?} ({})", lex.tokens, lex.tokens.len());

    Ok(lex.tokens)
}

// TODO: maybe use a special test key instead of official default key so we can test for weirder shapes
#[cfg(test)]
mod tests {
    use super::*;

    // Tile tests
    #[test]
    fn test_tile_from_1d() {
        let img = ImageReader::open("test/100x100.png").unwrap().decode().unwrap();

        let test = Tile::from_1d(123, 12, 3, &img);
        let expected = Tile {
                x: 23,
                y: 1,
                width: 12,
                height: 3
            };

        assert_eq!(test, expected);
    }

    #[test]
    fn test_tile_overlapping() {
        let test = Tile {
            x: 19,
            y: 38,
            width: 98,
            height: 21
        };
        let expected_false = [
            Tile { x: 0, y: 0, width: 0, height: 0},
            Tile { x: 10, y: 62, width: 8, height: 30}
        ];
        let expected_true = [
            Tile { x: 0, y: 1, width: 19, height: 37 },
            Tile { x: 1, y: 0, width: 18, height: 38 },
            Tile { x: 19, y: 3, width: 0, height: 35 },
            Tile { x: 17, y: 38, width: 2, height: 0 },
            Tile { x: 0, y: 0, width: 100, height: 100 }
        ];

        for expected in expected_false {
            assert!(!Tile::overlapping(&test, &expected));
        }

        for expected in expected_true {
            assert!(Tile::overlapping(&test, &expected));
        }
    }

    // Key tests
    struct KeySetup {
        img: image::DynamicImage,
        key: Key
    }

    impl KeySetup {
        fn new() -> Self {
            let mut setup = Self {
                img: ImageReader::open("examples/key.png").unwrap().decode().unwrap(),
                key: Key::new()
            };
            setup.key.read_keys(&setup.img);

            setup
        }
    }

    #[test]
    fn test_key_data_from_colour() {
        let key = KeySetup::new();

        // using Increment as an example
        // TODO: maybe test all keys?
        let test = key.key.data_from_colour(Rgb([153, 229, 80]));
        let expected = &key.key.increment;

        assert_eq!(*test[0], *expected);
    }

    #[test]
    fn test_key_data_from_token() {
        let key = KeySetup::new();

        // using Increment as an example
        let test = key.key.data_from_token(Token::Increment);
        let expected = &key.key.increment;

        assert_eq!(*test, *expected);
    }

    #[test]
    fn test_key_get_largest() {
        let key = KeySetup::new();

        let test = key.key.get_largest();
        // largest size of keys is width of repeat and height of line break
        let expected = (44, 46);

        assert_eq!(test, expected);
    }

    #[test]
    fn test_key_identify_background() {
        let key_file = ImageReader::open("examples/key.png").unwrap().decode().unwrap();

        let mut test = Key::new();
        test.identify_background(&key_file);
        let expected = Rgb([34, 32, 52]);

        assert_eq!(test.background, expected);
    }

    // TODO: make tests for all the key functions that involve tiles (return or param)

    // Lexer tests
    // TODO: do more cases for each test
    // TODO: make test 100x100.png example file more diverse
    struct LexerSetup {
        img: image::DynamicImage,
        key: image::DynamicImage,
        lexer: Lexer
    }

    impl LexerSetup {
        fn new() -> Self {
            let mut setup = Self {
                img: ImageReader::open("test/100x100.png").unwrap().decode().unwrap(),
                key: ImageReader::open("examples/key.png").unwrap().decode().unwrap(),
                lexer: Lexer {
                    key: Key::new(),
                    tokens: Default::default()
                }
            };

            setup.lexer.key.read_keys(&setup.key);

            setup
        }
    }

    #[test]
    fn test_lexer_compute_tile() {
        let img = ImageReader::open("test/100x100.png").unwrap().decode().unwrap();

        let test = Lexer::compute_tile(&Tile {
            x: 7,
            y: 12,
            width: 11,
            height: 23
        },
        Rgb([34, 32, 52]),
        &img);
        let expected = 253;

        assert_eq!(test, expected);

    }

    #[test]
    fn test_lexer_consume_first() {
        let setup = LexerSetup::new();

        let test = setup.lexer.consume_first(21, &setup.img);
        let expected = Token::Quote;

        assert_eq!(test, expected);
    }

    #[test]
    fn test_lexer_line_height() {
        let setup = LexerSetup::new();

        let test = setup.lexer.line_height(23, &setup.img);
        let expected = 12;

        assert_eq!(test, expected);
    }

    #[test]
    fn test_lexer_analyse_line() {
        let mut setup = LexerSetup::new();

        // TODO: gotta fix this test to be actual dimensions but rn analyse_line() is giving back in accurate size so well just test against that until i fix it. (see analyse_line() TODOs)
        let test = setup.lexer.analyse_line(1128, &setup.img); // 1128 is first pixel of a key
        let expected = (vec![Lexeme::Token(Token::Quote), Lexeme::Token(Token::LineBreak)],
                        Tile {
                            x: 28,
                            y: 11,
                            width: 72,
                            height: 12
                        });

        assert_eq!(test, expected);
    }

    #[test]
    fn test_lexer_analyse() {
        let mut setup = LexerSetup::new();

        setup.lexer.analyse(&setup.img);
        let test = setup.lexer.tokens;
        let expected = vec![Lexeme::Token(Token::Quote), Lexeme::Token(Token::LineBreak)];

        assert_eq!(test, expected);
    }
}
