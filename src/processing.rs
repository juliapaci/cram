use image::io::Reader as ImageReader;
use image::GenericImageView;
use image::Rgb;
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
}

impl Key {
    fn new() -> Self {
        Self {
            number: Rgb([0, 0, 0]),
            string: Rgb([0, 0, 0]),
            background: Rgb([0, 0, 0])
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

    pub fn identify_background(image: &image::DynamicImage) -> Rgb<u8> {
        let mut histogram: HashMap<Rgb<u8>, usize> = HashMap::new();
        for pixel in image.to_rgb8().pixels() {
            let mut counter = histogram.entry(*pixel).or_insert(0);
            *counter += 1;
        }

        let background = histogram
            .iter()
            .max_by_key(|(_, &count)| count)
            .unwrap_or((&Rgb([0, 0, 0]), &0));

        *background.0
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
    lex.key.background = Lexer::identify_background(&image);
    println!("background colour: {}, {}, {}", lex.key.background[0], lex.key.background[1], lex.key.background[2]);

    // image.save("output.png")?;

    Ok(())
}
