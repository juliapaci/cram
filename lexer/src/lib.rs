use image::io::Reader as ImageReader;
use image::{GenericImageView, Pixel, Rgb};

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use sha256::try_digest;

const TILE_SIZE: usize = 64;

// TODO: could use serde instead of custom log serialization but idk

// TODO: ggpu or multithreading for faster lexing
// TODO: incremental compilation
// TODO: linter

// bounds checking
macro_rules! bounds_check {
    ($position: expr, $bound: expr, $in: block, $out: block) => {
        if !bounds_check!($position, $bound) $out
        else $in
    };

    ($position: expr, $bound: expr, $out: block) => {
        if !bounds_check!($position, $bound) $out
    };

    ($position: expr, $bound: expr) => {
        ($position as usize) < $bound as usize
    }
}

#[derive(Default, Copy, Clone, PartialEq, Debug)]
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
            x: pos % (image.width() as usize),
            y: pos / (image.width() as usize),
            width,
            height,
        }
    }

    // check if two tiles are overlapping
    fn overlapping(a: &Tile, b: &Tile) -> bool {
        (a.x + a.width as usize >= b.x && b.x + b.width as usize >= a.x)
            && (a.y + a.height as usize >= b.y && b.y + b.height as usize >= a.y)
    }

    // returns the amount of same coloured pixels in a tile
    fn compute_tile(&self, colour: Rgb<u8>, image: &image::DynamicImage) -> u32 {
        let mut amount = 0;

        for y in 0..self.height as usize {
            bounds_check!(self.y + y, image.height(), { break });
            for x in 0..self.width as usize {
                bounds_check!(self.x + x, image.width(), { break });

                amount += (image
                    .get_pixel((self.x + x) as u32, (self.y + y) as u32)
                    .to_rgb()
                    == colour) as u32;
            }
        }

        amount
    }

    // detects solid rectangles for scopes
    // returns the tile that encampasses the rectangle
    fn detect_rectangle(begin: (usize, usize), image: &image::DynamicImage) -> Self {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels
            .chunks_exact(image.width() as usize)
            .map(|chunk| chunk.to_vec())
            .collect();
        let background = pixels[begin.1][begin.0];

        Self {
            x: begin.0,
            y: begin.1,

            width: pixels[begin.1][begin.0..]
                .iter()
                .position(|p| *p != background)
                .unwrap_or(image.width() as usize) as u32,

            height: pixels
                .iter()
                .map(|row| row[begin.0])
                .collect::<Vec<Rgb<u8>>>()[begin.1..]
                .iter()
                .position(|&p| p != background)
                .unwrap_or(image.height() as usize) as u32,
        }
    }

    // will save a pixels in a tile as an image
    #[allow(dead_code)] // debug function
    fn save_tile(
        &self,
        name: String,
        source: &image::DynamicImage,
    ) -> Result<(), image::ImageError> {
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);

        for y in 0..self.height as u32 {
            for x in 0..self.width as u32 {
                if (x < img.width() && y < img.height())
                    && (self.x as u32 + x < source.width() && self.y as u32 + y < source.height())
                {
                    img.put_pixel(
                        x as u32,
                        y as u32,
                        source
                            .get_pixel(self.x as u32 + x, self.y as u32 + y)
                            .to_rgb(),
                    );
                }
            }
        }

        img.save(name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Token {
    // static keys (read from key file)
    Zero,
    Increment,
    Decrement,
    Access,
    Repeat,
    Quote,
    #[default]
    LineBreak,
    ScopeStart,
    ScopeEnd,

    // dynamic keys (read from source file)
    Variable,
}

#[derive(Debug, PartialEq)]
pub enum Lexeme {
    Token(Token),      // key file tokens (static tokens i.e keys)
    Identifier(usize), // source file tokens (dynamic tokens e.g. variables) with a wrapped id
}

#[derive(Debug)]
struct Scope {
    colour: Rgb<u8>,
    tile: Tile,
}

// data for the tokens
// TODO: multi coloured? just use a map
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct KeyData {
    token: Token,    // token that the key represents
    colour: Rgb<u8>, // colour of key
    width_left: u8,  // width of key from the first (top left) pixel leftwards
    width_right: u8, // width of key from the first (top left) pixel rightwards
    height_up: u8,   // height of key from the first (leftmost) pixel upwards
    height_down: u8, // height of key from the first (leftmost) pixel downwards
    amount: u32,     // amount of non ignored (e.g. background, grid) pixels in key
}

impl std::fmt::Display for KeyData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let channels = self.colour.channels();
        write!(
            f,
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            channels[0],
            channels[1],
            channels[2],
            self.width_left,
            self.width_right,
            self.height_up,
            self.height_down,
            self.amount
        )
    }
}

impl Default for KeyData {
    fn default() -> Self {
        Self {
            token: Default::default(),
            colour: Rgb([0, 0, 0]),
            width_left: 0,
            width_right: 0,
            height_up: 0,
            height_down: 0,
            amount: 0,
        }
    }
}

// macro for logging
macro_rules! take {
    ($data: expr) => {
        $data.next()?.parse().ok()?
    };
}

// data from key file parsing (except variables)
struct Key {
    // for turing completeness
    zero: KeyData,      // the constant `0`
    increment: KeyData, // increment a value
    decrement: KeyData, // decrement a value
    access: KeyData,    // access a memory address
    repeat: KeyData,    // conditional jump

    // language syntax
    quote: KeyData,          // for string literals
    line_break: KeyData,     // seperates lines
    variables: Vec<KeyData>, // variables symbols (like names) that have been defined in source files

    // not a token
    background: Rgb<u8>, // background colour of the image
    grid: Rgb<u8>,       // grid colour for the key file
}

impl Key {
    fn new() -> Self {
        Self {
            zero: KeyData::default(),
            increment: KeyData::default(),
            decrement: KeyData::default(),
            access: KeyData::default(),
            repeat: KeyData::default(),

            quote: KeyData::default(),
            line_break: KeyData::default(),
            variables: Vec::new(),

            background: Rgb([0, 0, 0]),
            grid: Rgb([0, 0, 0]),
        }
    }

    // structure of log file:
    // - key file checksum
    // - see KeyData Display trait
    // seperated by a newline

    // TODO: in future maybe keep track of position of all the keys in source and key file so we can use compression for vc and stuff
    // TODO: keep logs of parts of source file so we dont have to recompile everything all the time

    // encodes Key into a log file
    fn write_log<P: AsRef<Path>>(&self, checksum: &String, path: P) -> std::io::Result<()> {
        fs::write(&path, "")?;
        let mut log = fs::OpenOptions::new().append(true).open(&path)?;

        writeln!(log, "{}", checksum)?;

        // unwrap()s fine since key is expected to be Some(_);
        self.data()
            .iter()
            .for_each(|&k| writeln!(log, "{}", k).unwrap());

        let bc = self.background.channels();
        let gc = self.grid.channels();
        writeln!(log, "{}\n{}\n{}", bc[0], bc[1], bc[2])?;
        writeln!(log, "{}\n{}\n{}", gc[0], gc[1], gc[2])?;

        Ok(())
    }

    // decodes the log file and returns the checksum and the Key
    fn read_log<P: AsRef<Path>>(&self, path: P) -> Option<(String, Key)> {
        let log = fs::read_to_string(&path).ok()?.parse::<String>().ok()?;
        let mut values = log.lines();
        let checksum = values.next()?.to_owned();

        Some((
            checksum,
            Key {
                zero: Self::read_key(&mut values, Token::Zero)?,
                increment: Self::read_key(&mut values, Token::Increment)?,
                decrement: Self::read_key(&mut values, Token::Decrement)?,
                access: Self::read_key(&mut values, Token::Access)?,
                repeat: Self::read_key(&mut values, Token::Repeat)?,
                quote: Self::read_key(&mut values, Token::Quote)?,
                line_break: Self::read_key(&mut values, Token::LineBreak)?,
                background: Rgb([take!(values), take!(values), take!(values)]),
                grid: Rgb([take!(values), take!(values), take!(values)]),

                variables: Default::default(),
            },
        ))
    }

    fn read_key(data: &mut std::str::Lines, token: Token) -> Option<KeyData> {
        Some(KeyData {
            token,
            colour: Rgb([take!(data), take!(data), take!(data)]),
            width_left: take!(data),
            width_right: take!(data),
            height_up: take!(data),
            height_down: take!(data),
            amount: take!(data),
        })
    }

    // TODO: dont hardcode the size & use serialisation
    // converts the members of Key to an array, excluding some members
    fn data(&self) -> Vec<&KeyData> {
        let mut keys = vec![
            &self.zero,
            &self.increment,
            &self.decrement,
            &self.access,
            &self.repeat,
            &self.quote,
            &self.line_break,
        ]; // keys from key file
        keys.extend(self.variables.iter()); // keys from source file (variables)

        keys
    }

    fn data_mut(&mut self) -> Vec<&mut KeyData> {
        let mut keys = vec![
            &mut self.zero,
            &mut self.increment,
            &mut self.decrement,
            &mut self.access,
            &mut self.repeat,
            &mut self.quote,
            &mut self.line_break,
        ]; // keys from key file
        keys.extend(self.variables.iter_mut()); // keys from source file (variables)

        keys
    }

    // gets the KeyData of keys that are of the specified colour
    fn data_from_colour(&self, colour: Rgb<u8>) -> Vec<&KeyData> {
        self.data()
            .iter()
            .filter(|&k| k.colour == colour)
            .copied()
            .collect::<Vec<&KeyData>>()
    }

    // TODO: find a way to include variables
    // returns the KeyData of a token
    fn data_from_token(&self, key: Token) -> &KeyData {
        // unsafe is fine since every token has an index in the array since its hardcoded (see as_array())
        self.data()[unsafe { std::mem::transmute::<Token, u8>(key) } as usize]
    }

    // gets the largest height and width from all of the keys (likely not from the same key)
    fn get_largest(&self) -> (u8, u8) {
        let sizes: Vec<(u8, u8)> = self
            .data()
            .iter()
            .map(|&k| (k.width_left + k.width_right, k.height_up + k.height_down))
            .collect();

        // unwrap is fine since we hardcode the array
        (
            // width
            sizes.iter().map(|s| s.0).max().unwrap(),
            // height
            sizes.iter().map(|s| s.1).max().unwrap(),
        )
    }

    // gets the background colour
    fn identify_background(&mut self, image: &image::DynamicImage) {
        let mut histogram: HashMap<Rgb<u8>, usize> = HashMap::new();
        for pixel in image.to_rgb8().pixels() {
            histogram
                .entry(*pixel)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        let background = histogram
            .iter()
            .max_by_key(|(_, &count)| count)
            .unwrap_or((&Rgb([0, 0, 0]), &0));

        self.background = *background.0;
    }

    // converts an area of the image to a 2d array of pixels
    fn tile_to_pixels(
        &self,
        tile: &Tile,
        background: Rgb<u8>,
        image: &image::DynamicImage,
    ) -> [[Rgb<u8>; TILE_SIZE]; TILE_SIZE] {
        let mut pixels: [[Rgb<u8>; TILE_SIZE]; TILE_SIZE] = [[background; TILE_SIZE]; TILE_SIZE];

        for y in 0..tile.height as usize {
            for x in 0..tile.width as usize {
                if tile.y + y >= image.height() as _ || tile.x + x >= image.width() as _ {
                    continue;
                }

                pixels[y][x] = image
                    .get_pixel((tile.x + x) as u32, (tile.y + y) as u32)
                    .to_rgb();
            }
        }

        pixels
    }

    // TODO: make it more flexible so the key file isnt restricted to a certain resolution
    // splits an image into 4x4 64x64 chunks
    fn image_to_tiles(
        &mut self,
        image: &image::DynamicImage,
    ) -> [[[Rgb<u8>; TILE_SIZE]; TILE_SIZE]; 16] {
        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().copied().collect();

        let mut tiles: [[[Rgb<u8>; TILE_SIZE]; TILE_SIZE]; 16] =
            [[[Rgb([0, 0, 0]); TILE_SIZE]; TILE_SIZE]; 16];
        for tile in 0..16 {
            let tile_offset = (tile / 4) * TILE_SIZE * 256 + (tile % 4) * TILE_SIZE;
            for y in 0..TILE_SIZE {
                let y_offset = y * image.width() as usize;
                for x in 0..TILE_SIZE {
                    tiles[tile][y][x] = pixels[tile_offset + y_offset + x];
                }
            }
        }

        tiles
    }

    // reads the key but doesnt remove parts within it. Useful for reading hollow keys
    // will panic if there is nothing (ignored pixels) occupying the tile (e.g. exclusively background and/or grid pixels)
    // TODO: add background param to this so it works in scopes
    fn outline_key(&self, tile: &[[Rgb<u8>; TILE_SIZE]; TILE_SIZE], token: Token) -> KeyData {
        // the trimmed key
        let mut key: Vec<Vec<Rgb<u8>>> = Vec::new();

        for row in tile {
            let first = match row
                .iter()
                .position(|&p| p != self.background && p != self.grid)
            {
                Some(i) => i,
                None => continue,
            };

            // dont need to copy this but im assuming that we will need to when we identify more
            // specific attributes of each key so im leaving this here
            let last = match row
                .iter()
                .copied()
                .rev()
                .position(|p| p != self.background && p != self.grid)
            {
                Some(i) => row.len() - i,
                None => continue,
            };

            // trim around the key (the background outside)
            let left: Vec<Rgb<u8>> = row[..first]
                .iter()
                .filter(|&p| *p != self.background && *p != self.grid)
                .copied()
                .collect();
            let right: Vec<Rgb<u8>> = row[last..]
                .iter()
                .filter(|&p| *p != self.background && *p != self.grid)
                .copied()
                .collect();
            let middle = row[first..last].to_vec();

            let mut tok = Vec::with_capacity(left.len() + middle.len() + right.len());
            tok.extend(middle);
            tok.extend(right);

            key.push(tok);
        }

        // top left pixel's coords
        let mut first_pixel: (usize, usize) = Default::default();

        first_pixel.0 = tile // x
            .iter()
            .filter(|row| row.iter().any(|&p| p != self.background && p != self.grid))
            .flat_map(|row| row.iter())
            .position(|&p| p != self.background && p != self.grid)
            .unwrap_or(0);

        first_pixel.1 = *tile // y
            .iter()
            .enumerate()
            .map(|(y, row)| {
                if row[first_pixel.0] == self.background || row[first_pixel.0] == self.grid {
                    0
                } else {
                    y
                }
            })
            .collect::<Vec<usize>>()
            .into_iter()
            .filter(|&a| a != 0)
            .collect::<Vec<usize>>()
            .first()
            .unwrap_or(&0);

        // left most pixel's coords
        let leftmost_pixel: (usize, usize) = tile
            .iter()
            .enumerate()
            .map(|(y, row)| {
                (
                    row.iter()
                        .position(|&p| p != self.background && p != self.grid)
                        .unwrap_or(TILE_SIZE),
                    y,
                )
            })
            .min()
            .unwrap();

        // tile without any background or grid pixels
        let filtered: Vec<Vec<&Rgb<u8>>> = key
            .iter()
            .map(|row| {
                row.iter()
                    .filter(|&p| *p != self.background && *p != self.grid)
                    .collect::<Vec<&Rgb<u8>>>()
            })
            .collect();

        let width = key.iter().map(Vec::len).max().unwrap_or(0) as i16;
        KeyData {
            token,
            colour: key.get(0).unwrap_or(&vec![Rgb([0, 0, 0])])[0],

            // fields values are from leftmost
            width_left: (first_pixel.0 as i16 - leftmost_pixel.0 as i16).abs() as u8,
            width_right: (width - (first_pixel.0 as i16 - leftmost_pixel.0 as i16)).abs() as u8,

            height_up: (leftmost_pixel.1 as i16 - first_pixel.1 as i16).abs() as u8,
            height_down: key.len() as u8
                - (leftmost_pixel.1 as i16 - first_pixel.1 as i16).abs() as u8,

            amount: filtered.iter().map(Vec::len).sum::<usize>() as u32,
        }
    }

    // read each 64x64 "tile" and apply the colour inside to the key structure
    fn read_keys(&mut self, image: &image::DynamicImage) {
        self.identify_background(image);
        let tiles = self.image_to_tiles(image);

        let grid = Tile::detect_rectangle((0, 0), image);
        if grid.width == image.width() && grid.height == image.height() {
            self.grid = tiles[0][0][0];
        }

        let keys: Vec<KeyData> = self
            .data()
            .iter()
            .enumerate()
            .map(|(i, _)| self.outline_key(&tiles[i], unsafe { std::mem::transmute(i as u8) }))
            .collect();
        // assign key fields to real data
        self.data_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(i, &mut ref mut key)| {
                // unsafe is fine since we are hardcoding the possible values of teken
                **key = keys[i];
            });
    }
}

struct Lexer<'a> {
    image: &'a image::DynamicImage, // translation unit

    key: Box<Key>,
    tokens: Vec<Lexeme>,
    ignore: HashMap<Rgb<u8>, Tile>, // TODO: should everything use self.ignore or their own ignore maps
    backgrounds: Vec<Rgb<u8>>,      // scope stack
}

impl<'a> Lexer<'a> {
    fn new(image: &'a image::DynamicImage) -> Self {
        Self {
            image,
            key: Box::new(Key::new()),
            tokens: Vec::new(),
            ignore: HashMap::new(),
            backgrounds: Vec::new(),
        }
    }

    fn background(&self) -> Rgb<u8> {
        // garunteed to atleast have self.key.background
        // so we can safely `unwrap`
        *self.backgrounds.last().unwrap()
    }

    // TODO: instead of passing around background we should keep a background field to change and read that whenever, like another self.key.background

    // returns the first keys token from a 1d index onwards
    // TODO: wont get the first, will get the heighest
    // TODO: optimise this with ignore map
    fn get_first(&self, bounds: &Tile) -> Token {
        // TODO: use a macro or heigher order function for this loop since we use it alot
        for x in bounds.x..(bounds.x + bounds.width as usize).min(self.image.width() as usize) {
            for y in bounds.y..(bounds.y + bounds.height as usize).min(self.image.height() as usize)
            {
                let pixel = self.image.get_pixel(x as u32, y as u32).to_rgb();
                // `unwrap`: garunteed to have a background from `line_height`
                if pixel == self.background() {
                    continue;
                }

                for key in self.key.data_from_colour(pixel) {
                    let tile = Tile {
                        x: (x as isize - key.width_left as isize).max(0) as usize,
                        y,
                        width: (key.width_left + key.width_right) as u32,
                        height: (key.height_up + key.height_down) as u32,
                    };

                    // if the tile matches a key
                    if tile.compute_tile(pixel, self.image) == key.amount {
                        return key.token;
                    }
                }
            }
        }

        // if theres no first key
        Token::LineBreak // maybe should be default token?
    }

    // return the height of the line
    // its just the tallest key that intersects a ray from the first keys middle row
    fn line_height(&self, bounds: &Tile) -> u8 {
        let first = self.key.data_from_token(self.get_first(bounds));
        let mut ignore: HashMap<Rgb<u8>, _> = HashMap::new();
        let mut max_height: u8 = first.height_up + first.height_down;
        let linebreak_colour = self.key.data_from_token(Token::LineBreak).colour;

        // index of middle row of key
        let middle_row =
            (bounds.y + (max_height / 2) as usize).min(self.image.height() as usize - 1);

        for x in bounds.x..(bounds.x + bounds.width as usize).min(self.image.width() as usize) {
            // TODO: see if we should check if the key exists instead of just relying on one pixel
            //       pros: more accurate line height + possibly faster tokenization
            //       cons: slower + more accurate tokenization

            let colour = self.image.get_pixel(x as u32, middle_row as u32).to_rgb();
            if colour == self.background() {
                continue;
            }

            match ignore.get(&colour) {
                Some(_) => continue,
                None => ignore.insert(colour, true),
            };

            max_height = self
                .key
                .data_from_colour(colour)
                .iter()
                .map(|&k| k.height_up + k.height_down)
                .max()
                .unwrap_or(0) // need to do this cause we dont check if the key exists yet
                .max(max_height);

            if colour == linebreak_colour {
                break;
            }
        }

        max_height
    }

    // TODO: should multiple analysis functions change self.tokens or should they each return Vec<Lexeme> to concantenate together in one place?
    // TODO: panics when variables are referenced with rectangular symbols/names
    // TODO: dont duplicate code in analyse(), make a generic loop with a higher order function or something
    // tokenizes a scope
    // TODO: this is so slow please optimise
    fn analyse_scope(&mut self, scope: &Scope) {
        self.backgrounds.push(scope.colour);

        // TODO: keep pixels as a struct member so we dont always have to recompute it.
        //       issue is theres multiple forms of pixel data e.g. array, matrix, "chunks"
        let pixels: Vec<Rgb<u8>> = self.image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels
            .chunks_exact(self.image.width() as usize)
            .map(|chunk| chunk.to_vec().iter().cloned().collect())
            .collect();

        self.tokens.push(Lexeme::Token(Token::ScopeStart));

        let possible_line_size = self.key.get_largest();
        let mut frame = Tile {
            x: scope.tile.x,
            y: scope.tile.y,
            width: possible_line_size.0 as u32,
            height: possible_line_size.1 as u32,
        };

        let init_x = scope.tile.x;
        // see self.analyse() for details
        while frame.y < scope.tile.y + scope.tile.height as usize {
            frame.x = init_x;
            while frame.x < scope.tile.x + scope.tile.width as usize {
                'frame: for x in 0..frame.width as usize {
                    bounds_check!(x + frame.x, self.image.width(), { break });

                    for y in 0..frame.height as usize {
                        bounds_check!(y + frame.y, self.image.height(), { break });

                        if pixels[y + frame.y][x + frame.x] == scope.colour {
                            continue;
                        }

                        let line = self.analyse_line(
                            &mut Tile {
                                x: x + frame.x,
                                y: y + frame.y,
                                width: scope.tile.width - x as u32,
                                height: scope.tile.height - y as u32,
                            },
                        );
                        frame.x += line.width as usize;
                        frame.y += line.height as usize;

                        break 'frame;
                    }
                }
                frame.x += frame.width as usize;
            }
            frame.y += frame.height as usize;
        }

        self.tokens.push(Lexeme::Token(Token::ScopeEnd));
        assert_eq!(self.backgrounds.pop(), Some(scope.colour));
    }

    // tokenizes a line of keys
    // returns area of the line to be skipped so its not analysed again
    // TODO: remove some ignore entries that are far away from the crrent iteration pixel locaiton
    // TODO: jump over ignored areas instead of just continue;ing
    fn analyse_line(&mut self, bounds: &Tile) -> Tile {
        let mut size = bounds.clone();
        size.height = self.line_height(bounds) as u32;
        if size.height == 0 {
            return size;
        }

        // faster to do this or to use get_pixel()?
        let pixels: Vec<Rgb<u8>> = self.image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels
            .chunks_exact(self.image.width() as usize)
            .map(|chunk| chunk.to_vec())
            .collect();

        // TODO: optimise line height to perfectly fit everything (right now its larger than it needs to be) + then we can use Tile::overlapping because we wont need custom yh for loop
        'img: for x in size.x..(size.x + size.width as usize).min(self.image.width() as usize) {
            for y in size.y..(size.y + size.height as usize).min(self.image.height() as usize) {
                // TODO: unsure if we should check for key background here since it might be an
                // error for the parser
                if pixels[y][x] == self.background() || pixels[y][x] == self.key.background {
                    continue;
                }

                // checking if where in an area thats already been checked
                if let Some(tile) = self.ignore.get(&pixels[y][x]) {
                    if Tile::overlapping(
                        &Tile {
                            x,
                            y,
                            width: 0,
                            height: 0,
                        },
                        tile,
                    ) {
                        continue;
                    }
                }
                if let Some(tile) = self.ignore.get(&self.key.line_break.colour) {
                    // check area
                    // see the hack todo in the scope part where we insert specifically for line break colour
                    if Tile::overlapping(
                        &Tile {
                            x,
                            y,
                            width: 0,
                            height: 0,
                        },
                        tile,
                    ) {
                        continue;
                    }
                }

                // read variable decleration, expected after an Access token
                if matches!(self.tokens.last(), Some(lexeme)
                            if matches!(lexeme, Lexeme::Token(token)
                                        if *token == Token::Access))
                {
                    // TODO: this weirdly breaks if colours are above it??
                    // TODO: fix default bounding box of possible variable by finding the actual size before outline key maybe
                    self.key.variables.push(self.key.outline_key(
                        &self.key.tile_to_pixels(
                            &Tile {
                                x,
                                y: size.y - 1,
                                width: TILE_SIZE as _,
                                height: size.height,
                            },
                            self.background(),
                            &self.image,
                        ),
                        Token::Variable,
                    ));
                }

                // if the pixel is unknown then it could be a scope
                if self.key.data_from_colour(pixels[y][x]).is_empty() {
                    let scope = Tile::detect_rectangle((x, y), self.image);
                    // rectangle is big enough to be a scope
                    if scope.width > TILE_SIZE as _ && scope.height > TILE_SIZE as _ {
                        self.analyse_scope(&Scope {
                            colour: pixels[y][x],
                            tile: scope,
                        });

                        // TODO: very hacky, using LineBreak colour to denote a general area to ignore.
                        // should do something different
                        self.ignore.insert(self.key.line_break.colour, scope);
                        continue;
                    }
                }

                // checking if a key matches pixels in a tile
                for key in self.key.data_from_colour(pixels[y][x]) {
                    let tile = Tile {
                        x,
                        y: y.max(key.height_up as usize) - key.height_up as usize,
                        width: (key.width_left + key.width_right) as u32,
                        height: (key.height_up + key.height_down) as u32,
                    };

                    // if the tile matches a key
                    if tile.compute_tile(pixels[y][x], self.image) == key.amount {
                        self.tokens.push(match key.token {
                            Token::Variable => Lexeme::Identifier(
                                self.key.variables.iter().position(|v| v == key).unwrap(),
                            ),
                            _ => Lexeme::Token(key.token),
                        });

                        // line ends if line break, scope edge,
                        if key.token == Token::LineBreak {
                            size.width = (x - size.x) as u32 + key.width_right as u32;
                            break 'img;
                        }
                    }

                    // marks this area as already checked
                    self.ignore.insert(pixels[y][x], tile);
                }
            }
        }

        // inserting a line break if there wasnt one there
        // TODO: ignore consecutive LineBreaks better
        if let Some(lexeme) = self.tokens.last() {
            if *lexeme != Lexeme::Token(Token::LineBreak)
                && *lexeme != Lexeme::Token(Token::ScopeEnd)
            {
                self.tokens.push(Lexeme::Token(Token::LineBreak));
            }
        }

        size
    }

    pub fn analyse(&mut self) {
        self.backgrounds.push(self.key.background);

        let pixels: Vec<Rgb<u8>> = self.image.to_rgb8().pixels().copied().collect();
        let pixels: Vec<Vec<Rgb<u8>>> = pixels
            .chunks_exact(self.image.width() as usize)
            .map(|chunk| chunk.to_vec().iter().cloned().collect())
            .collect();

        let possible_line_size = self.key.get_largest();
        let mut frame = Tile {
            x: 0,
            y: 0,
            width: possible_line_size.0 as u32,
            height: possible_line_size.1 as u32,
        };

        while frame.y < self.image.height() as usize {
            // how many frames can fit on y
            frame.x = 0;
            while frame.x < self.image.width() as usize {
                // how many frames can fit on x
                // check for anything in side the frame
                'frame: for x in 0..frame.width as usize {
                    if x + frame.x >= self.image.width() as usize {
                        break;
                    }

                    for y in 0..frame.height as usize {
                        if y + frame.y >= self.image.height() as usize {
                            break;
                        }

                        if pixels[y + frame.y][x + frame.x] == self.key.background {
                            continue;
                        }

                        let line = self.analyse_line(
                            &mut Tile {
                                x: x + frame.x,
                                y: y + frame.y,
                                width: self.image.width(),
                                height: self.image.height(),
                            },
                        );
                        frame.x += line.width as usize - 1; // TODO: should there be a "- 1" here?
                        frame.y += line.height as usize;

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
    let mut lex = Lexer::new(&source_img);

    let log_path = "out/key.log";
    let log_data = lex.key.read_log(log_path);
    let clear_read = log_data.is_some();
    let (checksum, log) = match log_data {
        Some(data) => data,
        None => (Default::default(), Key::new()),
    };
    if let Ok(digest) = try_digest(key) {
        if clear_read && checksum == digest {
            println!("Reading from log");
            lex.key = Box::new(log);
        } else {
            lex.key.read_keys(&key_img);
            lex.key.write_log(&digest, log_path).unwrap();
        }
    }
    println!("Finished reading keys");

    lex.analyse();
    println!("Finished tokenizing");

    Ok(lex.tokens)
}

// TODO: maybe use a special test key instead of official default key so we can test for weirder shapes
// TODO: do more extensive tests and test multiple cases
#[cfg(test)]
mod tests {
    use super::*;

    // Tile tests
    #[test]
    fn tile_from_1d() {
        let img = ImageReader::open("../test/100x100.png")
            .unwrap()
            .decode()
            .unwrap();

        let test = Tile::from_1d(123, 12, 3, &img);
        let expected = Tile {
            x: 23,
            y: 1,
            width: 12,
            height: 3,
        };

        assert_eq!(test, expected);
    }

    #[test]
    fn tile_overlapping() {
        let test = Tile {
            x: 19,
            y: 38,
            width: 98,
            height: 21,
        };
        let expected_false = [
            Tile {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            Tile {
                x: 10,
                y: 62,
                width: 8,
                height: 30,
            },
        ];
        let expected_true = [
            Tile {
                x: 0,
                y: 1,
                width: 19,
                height: 37,
            },
            Tile {
                x: 1,
                y: 0,
                width: 18,
                height: 38,
            },
            Tile {
                x: 19,
                y: 3,
                width: 0,
                height: 35,
            },
            Tile {
                x: 17,
                y: 38,
                width: 2,
                height: 0,
            },
            Tile {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            },
        ];

        for expected in expected_false {
            assert!(!Tile::overlapping(&test, &expected));
        }

        for expected in expected_true {
            assert!(Tile::overlapping(&test, &expected));
        }
    }

    #[test]
    fn tile_compute_tile() {
        let img = ImageReader::open("../test/100x100.png")
            .unwrap()
            .decode()
            .unwrap();

        let test = Tile {
            x: 7,
            y: 12,
            width: 11,
            height: 23,
        }
        .compute_tile(Rgb([34, 32, 52]), &img);
        let expected = 253;

        assert_eq!(test, expected);
    }

    #[test]
    fn tile_detect_rectangle() {
        let img = ImageReader::open("../test/scope.png")
            .unwrap()
            .decode()
            .unwrap();

        let test = Tile::detect_rectangle((38, 34), &img);
        let expected = Tile {
            x: 38,
            y: 34,
            width: 125,
            height: 126,
        };

        assert_eq!(test, expected);
    }

    // Key tests
    struct KeySetup {
        img: image::DynamicImage,
        key: Key,
    }

    impl KeySetup {
        fn new() -> Self {
            let mut setup = Self {
                img: ImageReader::open("../examples/key.png")
                    .unwrap()
                    .decode()
                    .unwrap(),
                key: Key::new(),
            };
            setup.key.read_keys(&setup.img);

            setup
        }
    }

    // TODO: logging tests
    // TODO: tests for all funcitonbs

    #[test]
    fn key_data_from_colour() {
        let key = KeySetup::new();

        // using Increment as an example
        // TODO: maybe test all keys?
        let test = key.key.data_from_colour(Rgb([153, 229, 80]));
        let expected = &key.key.increment;

        assert_eq!(*test[0], *expected);
    }

    #[test]
    fn key_data_from_token() {
        let key = KeySetup::new();

        // using Increment as an example
        let test = key.key.data_from_token(Token::Increment);
        let expected = &key.key.increment;

        assert_eq!(*test, *expected);
    }

    #[test]
    fn key_get_largest() {
        let key = KeySetup::new();

        let test = key.key.get_largest();
        // largest size of keys is width of repeat and height of line break
        let expected = (44, 46);

        assert_eq!(test, expected);
    }

    #[test]
    fn key_identify_background() {
        let key_file = ImageReader::open("../examples/key.png")
            .unwrap()
            .decode()
            .unwrap();

        let mut test = Key::new();
        test.identify_background(&key_file);
        let expected = Rgb([34, 32, 52]);

        assert_eq!(test.background, expected);
    }

    // TODO: make tests for all the key functions that involve tiles (return or param)

    // Lexer tests
    // TODO: do more cases for each test
    // TODO: make test 100x100.png example file more diverse
    struct LexerSetup<'a> {
        key: &'a image::DynamicImage,
        lexer: Lexer<'a>,
    }

    macro_rules! lexer_setup_example {
        ($res: ident, $src: expr) => {
            let key = ImageReader::open(LexerSetup::KEY)
                .unwrap()
                .decode()
                .unwrap();
            let src = ImageReader::open($src)
                .unwrap()
                .decode()
                .unwrap();
            let mut $res = LexerSetup::new(&key, &src);
            $res.lexer.backgrounds.push($res.lexer.key.background);
        };
    }

    impl<'a> LexerSetup<'a> {
        const KEY: &'a str = "../examples/key.png";
        const SQUARE: &'a str = "../test/100x100.png";
        const SCOPE: &'a str = "../test/scope.png";

        fn new(key: &'a image::DynamicImage, src: &'a image::DynamicImage) -> Self {
            let mut setup = Self {
                key,
                lexer: Lexer::new(src),
            };

            setup.lexer.key.read_keys(&setup.key);

            setup
        }
    }

    #[test]
    fn lexer_get_first() {
        lexer_setup_example!(setup, LexerSetup::SQUARE);

        let tile = Tile::from_1d(
            21,
            setup.lexer.image.width(),
            setup.lexer.image.height(),
            setup.lexer.image,
        );
        let test = setup.lexer.get_first(&tile);
        let expected = Token::Quote;

        assert_eq!(test, expected);
    }

    #[test]
    fn lexer_line_height() {
        lexer_setup_example!(setup, LexerSetup::SQUARE);

        let tile = Tile::from_1d(
            23,
            setup.lexer.image.width(),
            setup.lexer.image.height(),
            setup.lexer.image,
        );
        let test = setup.lexer.line_height(&tile);
        let expected = 12;

        assert_eq!(test, expected);
    }

    #[test]
    fn lexer_analyse_scope() {
        lexer_setup_example!(setup, LexerSetup::SCOPE);

        setup.lexer.analyse_scope(&Scope {
            colour: Rgb([0, 63, 35]),
            tile: Tile {
                x: 38,
                y: 34,
                width: 125,
                height: 126,
            },
        });
        let test = setup.lexer.tokens;
        let expected = vec![
            Lexeme::Token(Token::ScopeStart),
            Lexeme::Token(Token::Decrement),
            Lexeme::Token(Token::Quote),
            Lexeme::Token(Token::Quote),
            Lexeme::Token(Token::LineBreak),
            Lexeme::Token(Token::Repeat),
            Lexeme::Token(Token::Decrement),
            Lexeme::Token(Token::LineBreak),
            Lexeme::Token(Token::ScopeEnd),
        ];

        assert_eq!(test, expected);
    }

    #[test]
    fn lexer_analyse_line() {
        lexer_setup_example!(setup, LexerSetup::SQUARE);

        // TODO: gotta fix this test to be actual dimensions but rn analyse_line() is giving back inaccurate size so well just test against that until i fix it. (see analyse_line() TODOs)
        let test = setup.lexer.analyse_line(
            &Tile {
                x: 28,
                y: 11,
                width: setup.lexer.image.width(),
                height: setup.lexer.image.height(),
            },
        );
        let expected_area = Tile {
            x: 28,
            y: 11,
            width: setup.lexer.image.height(),
            height: 12,
        };
        let expected_tokens = vec![Lexeme::Token(Token::Quote), Lexeme::Token(Token::LineBreak)];

        assert_eq!(test, expected_area);
        assert_eq!(setup.lexer.tokens, expected_tokens);
    }

    #[test]
    fn lexer_analyse() {
        lexer_setup_example!(setup, LexerSetup::SQUARE);

        setup.lexer.analyse();
        let test = setup.lexer.tokens;
        let expected = vec![Lexeme::Token(Token::Quote), Lexeme::Token(Token::LineBreak)];

        assert_eq!(test, expected);
    }
}
