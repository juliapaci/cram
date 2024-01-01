use image::io::Reader as ImageReader;
use image::{GenericImageView, Rgb};
use std::collections::HashMap;

// what the colours mean
struct Key {
    // for turing completeness
    zero: Rgb<u8>,          // the constant `0`
    increment: Rgb<u8>,     // increment a value
    decrement: Rgb<u8>,     // decrement a value
    access: Rgb<u8>,        // access a memory address
    repeat: Rgb<u8>,        // jump based on a condition

    // extras
    string: Rgb<u8>,        // for string literals

    // not a token
    ignore: Rgb<u8>,        // a colour to ignore
    background: Rgb<u8>,    // background colour of the image
    grid: Rgb<u8>,          // grid colour for the key file
}

impl Key {
    fn new() -> Self {
        Self {
            zero: Rgb([0, 0, 0]),
            increment: Rgb([0, 0, 0]),
            decrement: Rgb([0, 0, 0]),
            access: Rgb([0, 0, 0]),
            repeat: Rgb([0, 0, 0]),

            string: Rgb([0, 0, 0]),
            background: Rgb([0, 0, 0]),

            ignore: Rgb([0, 0, 0]),
            grid: Rgb([0, 0, 0]),
        }
    }

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
    fn identify_key_colour(&self, tile: &[[Rgb<u8>; 64]; 64]) -> Rgb<u8> {
        let pixels: Vec<&Rgb<u8>> = tile
            .iter()
            .flat_map(|row| {
                row.iter()
                    .filter(|&p| *p != self.background && *p != self.grid)
            })
            .collect();

        *pixels[0]
    }

    // read each 64x64 "tile" and apply the colour inside to the key structure
    fn read_keys(&mut self, image: &image::DynamicImage) {
        self.identify_background(image);

        let tiles = self.image_to_tiles(image);
        self.save_tiles(&tiles).unwrap();
        // TODO: find better way of finding key grid colour
        self.grid = tiles[0][0][0];

        // TODO: better wat of doing all these actions like macro or something?
        self.zero = self.identify_key_colour(&tiles[0]);
        self.increment = self.identify_key_colour(&tiles[1]);
        self.decrement = self.identify_key_colour(&tiles[2]);
        self.access = self.identify_key_colour(&tiles[3]);
        self.repeat = self.identify_key_colour(&tiles[4]);
        self.string = self.identify_key_colour(&tiles[5]);
    }
}

struct Lexer {
    key: Key,
    tokens: Vec<Vec<u8>>
}

impl Lexer {
    fn new() -> Self {
        Self {
            key: Key::new(),
            tokens: Vec::new()
        }
    }

    pub fn lex(image: &image::DynamicImage) {
        let tokens: Vec<u8> = Vec::new();

        for pixel in image.pixels() {

        }
    }
}
pub fn deserialize(file_path: &String) -> Result<(), image::ImageError>{
    let image = ImageReader::open(file_path)?.with_guessed_format()?.decode()?;

    // lex(&image);
    let mut lex = Lexer::new();
    lex.key.read_keys(&image);

    // image.save("output.png")?;

    Ok(())
}
