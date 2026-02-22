use image::{ImageBuffer, Rgb, RgbImage};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (width, height, max_iter) = match args.len() {
        1 => (800, 600, 100), // defaults
        4 => {
            let w: u32 = args[1].parse().expect("Invalid width");
            let h: u32 = args[2].parse().expect("Invalid height");
            let i: i32 = args[3].parse().expect("Invalid iterations");
            (w, h, i)
        }
        _ => {
            eprintln!("Usage: {} [width height max_iterations]", args[0]);
            eprintln!("Example: {} 800 600 100", args[0]);
            eprintln!("Using defaults: 800 600 100");
            (800, 600, 100)
        }
    };

    println!("Generating Mandelbrot fractal...");
    let mandelbrot_pixels =
        fractal_generator::generate_mandelbrot(width as i32, height as i32, max_iter);
    save_to_image(&mandelbrot_pixels, "mandelbrot.png", width, height);

    println!("Generating Julia fractal...");
    let julia_pixels =
        fractal_generator::generate_julia(width as i32, height as i32, max_iter, -0.7, 0.27015);
    save_to_image(&julia_pixels, "julia.png", width, height);

    println!("Generating Sierpinski triangle...");
    let sierpinski_pixels = fractal_generator::generate_sierpinski(width as i32, height as i32, 8);
    save_to_image(&sierpinski_pixels, "sierpinski.png", width, height);

    println!("Fractals saved to mandelbrot.png, julia.png, and sierpinski.png");
}

fn save_to_image(
    pixels: &Arc<Vec<fractal_generator::Pixel>>,
    filename: &str,
    width: u32,
    height: u32,
) {
    let mut img: RgbImage = ImageBuffer::new(width, height);

    for pixel in pixels.iter() {
        let x = pixel.px as u32;
        let y = pixel.py as u32;
        let color = Rgb([pixel.pr as u8, pixel.pg as u8, pixel.pb as u8]);
        img.put_pixel(x, y, color);
    }

    img.save(filename).expect("Failed to save image");
    println!("Saved {} ({}x{})", filename, width, height);
}
