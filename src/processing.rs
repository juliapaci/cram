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
    background: Rgb<u8>,    // background colour of the image

    // not a token
    ignore: Rgb<u8>,
    grid: Rgb<u8>,
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

    // read each 64x64 "tile" and apply the colour inside to the key structure
    fn read_keys(&mut self, image: &image::DynamicImage) {
        self.identify_background(image);

        let pixels: Vec<Rgb<u8>> = image.to_rgb8().pixels().map(|&p| p).collect();
        self.grid = pixels[0];

        let mut tiles: Vec<Rgb<u8>> = pixels
            .chunks_exact(64)
            .flat_map(|chunk| chunk.iter())
            .filter(|&&c| c != self.background && c != self.grid)
            .cloned()
            .collect();

        for i in tiles.iter() {
            println!("{}, {}, {}", i[0], i[1], i[2]);
        }
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
    println!("background colour: {}, {}, {}", lex.key.background[0], lex.key.background[1], lex.key.background[2]);

    // image.save("output.png")?;

    Ok(())
}
