#[cfg_attr(test, feature(test))]

extern crate image;
extern crate num_traits;
#[cfg(test)]
extern crate test;

use image::{FilterType, DynamicImage, GenericImage, RgbaImage};
use image::imageops::resize;
use image::Pixel;
use num_traits::sign::abs_sub;
use num_traits::ToPrimitive;
use std::cmp::{max, min};
use std::default::Default;

pub fn compare_images<I1, I2>(img1: &I1, img2: &I2, opt: &ComparisonOptions) -> Compare
    where I1: GenericImage<Pixel = image::Rgba<u8>> + 'static,
          I2: GenericImage<Pixel = image::Rgba<u8>> + 'static
{
    let (width1, height1) = img1.dimensions();
    let (width2, height2) = img2.dimensions();

    let width = max(width1, width2);
    let height = max(height1, height2);

    let img1 = resize(img1, width, height, FilterType::Nearest);
    let img2 = resize(img2, width, height, FilterType::Nearest);
    let mut img_out = RgbaImage::new(width, height);

    let mut mismatch_count = 0;

    for x in 0..width {
        for y in 0..height {
            let pixel1 = img1.get_pixel(x, y);
            let rgba1 = pixel_to_rgba(pixel1);

            let pixel2 = img2.get_pixel(x, y);
            let rgba2 = pixel_to_rgba(pixel2);
            let are_equals = compare_pixel(&rgba1, &rgba2, &img1, &img2, (x, y), opt);

            if are_equals {
                img_out.put_pixel(x, y, pixel1.to_rgba());
            } else {
                img_out.put_pixel(x, y, image::Rgba { data: [255, 0, 255, 255] });
                mismatch_count += 1;
            }
        }
    }

    Compare {
        image: DynamicImage::ImageRgba8(img_out),
        is_same_dimension: width1 == width2 && height1 == height2,
        mismatch_percent: (mismatch_count * 100).to_f64().unwrap() /
                          (width * height).to_f64().unwrap(),
    }
}

pub struct Compare {
    pub image: DynamicImage,
    pub is_same_dimension: bool,
    pub mismatch_percent: f64,
}

pub struct ComparisonOptions {
    ignore_antialiasing: bool,
    ignore_colors: bool,
    tolerance: Tolerance,
}

impl ComparisonOptions {
    pub fn new() -> ComparisonOptions {
        ComparisonOptions {
            ignore_antialiasing: false,
            ignore_colors: false,
            tolerance: Default::default(),
        }
    }

    pub fn ignore_nothing(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 0.0;
        self.tolerance.blue = 0.0;
        self.tolerance.green = 0.0;
        self.tolerance.red = 0.0;
        self.tolerance.min_brightness = 0.0;
        self.tolerance.max_brightness = 255.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_less(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 16.0;
        self.tolerance.blue = 16.0;
        self.tolerance.green = 16.0;
        self.tolerance.red = 16.0;
        self.tolerance.min_brightness = 16.0;
        self.tolerance.max_brightness = 240.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_antialiasing(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 32.0;
        self.tolerance.blue = 32.0;
        self.tolerance.green = 32.0;
        self.tolerance.red = 32.0;
        self.tolerance.min_brightness = 64.0;
        self.tolerance.max_brightness = 96.0;
        self.ignore_antialiasing = true;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_colors(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 16.0;
        self.tolerance.min_brightness = 16.0;
        self.tolerance.max_brightness = 240.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = true;
        self
    }
}

#[derive(Default)]
struct Tolerance {
    alpha: f32,
    max_brightness: f32,
    min_brightness: f32,
    red: f32,
    green: f32,
    blue: f32,
}

struct Rgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

fn compare_pixel<I1, I2>(pixel1: &Rgba,
                         pixel2: &Rgba,
                         img1: &I1,
                         img2: &I2,
                         position: (u32, u32),
                         opt: &ComparisonOptions)
                         -> bool
    where I1: GenericImage<Pixel = image::Rgba<u8>>,
          I2: GenericImage<Pixel = image::Rgba<u8>>
{
    if !is_similar(pixel1.a, pixel2.a, opt.tolerance.alpha) {
        false
    } else if opt.ignore_colors {
        is_pixel_brightness_similar(&pixel1, &pixel2, &opt.tolerance)
    } else if is_rgb_similar(&pixel1, &pixel2, &opt.tolerance) {
        true
    } else if opt.ignore_antialiasing &&
              (is_antialiased(pixel1, img1, &position, &opt.tolerance) ||
               is_antialiased(pixel2, img2, &position, &opt.tolerance)) {
        true
    } else {
        false
    }
}

fn get_brightness(rgba: &Rgba) -> f32 {
    0.3 * rgba.r + 0.59 * rgba.g + 0.11 * rgba.b
}

fn get_hue(rgba: &Rgba) -> f32 {
    let (r, g, b) = (rgba.r, rgba.g, rgba.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);

    if max == min {
        0.0 // achromatic
    } else {
        let d = max - min;

        let h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        h / 6.0
    }
}

fn is_antialiased<I>(p1: &Rgba, image: &I, p: &(u32, u32), tolerance: &Tolerance) -> bool
    where I: GenericImage<Pixel = image::Rgba<u8>>
{
    const DISTANCE: u32 = 1;

    let (width, height) = image.dimensions();
    let (x, y) = (p.0, p.1);

    let left = max(x - DISTANCE, 0);
    let right = min(x + DISTANCE + 1, width);
    let top = max(y - DISTANCE, 0);
    let bottom = min(y + DISTANCE + 1, height);

    let brightness1 = get_brightness(p1);
    let hue1 = get_hue(p1);
    let mut has_equivalent_sibling = 0;

    for x in left..right {
        for y in top..bottom {

            // ignore source pixel
            if x == p.0 && y == p.1 {
                continue;
            }

            let p2 = image.get_pixel(x, y);
            let p2 = pixel_to_rgba(&p2);
            let brightness2 = get_brightness(&p2);
            let hue2 = get_hue(&p2);

            if abs_sub(brightness1, brightness2) > tolerance.max_brightness {
                return true;
            }

            if abs_sub(hue1, hue2) > 0.3 {
                return true;
            }

            if is_rgb_same(&p1, &p2) {
                has_equivalent_sibling += 1;
            }
        }
    }

    has_equivalent_sibling < 2
}

fn is_pixel_brightness_similar(p1: &Rgba, p2: &Rgba, tolerance: &Tolerance) -> bool {
    let brightness1 = get_brightness(p1);
    let brightness2 = get_brightness(p2);
    is_similar(brightness1, brightness2, tolerance.min_brightness)
}

fn is_rgb_same(p1: &Rgba, p2: &Rgba) -> bool {
    p1.r == p2.r && p1.g == p2.g && p1.b == p2.b
}

fn is_similar(v1: f32, v2: f32, tolerance: f32) -> bool {
    abs_sub(v1, v2) <= tolerance
}

fn is_rgb_similar(p1: &Rgba, p2: &Rgba, t: &Tolerance) -> bool {
    is_similar(p1.r, p2.r, t.red) && is_similar(p1.g, p2.g, t.green) &&
    is_similar(p1.b, p2.b, t.blue)
}

fn pixel_to_rgba<P: Pixel>(pixel: &P) -> Rgba {
    let rgb = pixel.to_rgba();
    Rgba {
        r: rgb.data[0].to_f32().unwrap(),
        g: rgb.data[1].to_f32().unwrap(),
        b: rgb.data[2].to_f32().unwrap(),
        a: rgb.data[3].to_f32().unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;
    use test::Bencher;

    #[bench]
    fn bench_compare_images(b: &mut Bencher) {
        let img1 = &image::open(&Path::new("./examples/people1.jpg"))
            .expect("unable to load people1.jpg");

        let img2 = &image::open(&Path::new("./examples/people2.jpg"))
            .expect("unable to load people2.jpg");

        let opts = &ComparisonOptions::new();

        b.iter(|| compare_images(img1, img2, opts));
    }
}