use image::Pixel;
use image::{DynamicImage, GenericImage, RgbaImage};
use num_traits::Signed;
use rayon::prelude::*;
use std::cmp::{max, min};
use std::default::Default;
use std::sync::atomic::{AtomicUsize, Ordering};

type Rgba = image::Rgba<u8>;

pub fn compare_images<I1, I2>(img1: &I1, img2: &I2, opt: &ComparisonOptions) -> Compare
where
    I1: GenericImage<Pixel = Rgba> + 'static + std::marker::Sync,
    I2: GenericImage<Pixel = Rgba> + 'static + std::marker::Sync,
{
    let (width, height) = width_height_from_2_images(img1, img2);
    let mut img_out = RgbaImage::new(width, height);
    let mismatch_count = AtomicUsize::new(0);

    img_out
        .par_chunks_mut(4)
        .enumerate()
        .for_each(|(index, pixel)| {
            let pixel = image::Rgba::from_slice_mut(pixel);
            let (x, y) = xy_from_index(width, index as u32);
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);
            let are_equals = compare_pixel(&pixel1, &pixel2, img1, img2, (x, y), opt);

            if are_equals {
                *pixel = pixel1;
            } else {
                *pixel = *Rgba::from_slice(&[255, 0, 255, 255]);
                mismatch_count.fetch_add(1, Ordering::SeqCst);
            }
        });

    let mismatch_count = mismatch_count.load(Ordering::SeqCst) as u32;

    Compare {
        image: DynamicImage::ImageRgba8(img_out),
        mismatch_percent: (mismatch_count * 100) as f64 / (width * height) as f64,
    }
}

/// Compare 2 images and return the mismatch percentage based on the number of pixels that are different.`.
///
/// # Examples
///
/// ```
/// extern crate image;
/// extern crate rust_resemble;
///
/// use rust_resemble::*;
/// use std::path::Path;
///
/// fn main() {
///     let img1 = &image::open(&Path::new("./examples/people1.jpg"))
///         .expect("unable to load people1.jpg");
///
///     let img2 = &image::open(&Path::new("./examples/people2.jpg"))
///         .expect("unable to load people2.jpg");
///
///     let opts = &ComparisonOptions::new();
///
///     let mismatch_percent = get_mismatch_percent(img1, img2, opts);
///     assert_eq!(mismatch_percent, 97.1228);
/// }
/// ```
pub fn get_mismatch_percent<I1, I2>(img1: &I1, img2: &I2, opt: &ComparisonOptions) -> f64
where
    I1: GenericImage<Pixel = Rgba> + 'static + std::marker::Sync,
    I2: GenericImage<Pixel = Rgba> + 'static + std::marker::Sync,
{
    let (width, height) = width_height_from_2_images(img1, img2);

    let mismatch_count: u64 = (0..width * height)
        .into_par_iter()
        .map(|index| {
            let (x, y) = xy_from_index(width, index);
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);
            let are_equals = compare_pixel(&pixel1, &pixel2, img1, img2, (x, y), opt);

            if are_equals {
                0u64
            } else {
                1u64
            }
        })
        .sum();

    let mismatch_percent = (mismatch_count * 100) as f64 / (width * height) as f64;
    mismatch_percent
}

pub struct Compare {
    pub image: DynamicImage,
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
        self.tolerance.alpha = 0;
        self.tolerance.blue = 0;
        self.tolerance.green = 0;
        self.tolerance.red = 0;
        self.tolerance.min_brightness = 0.0;
        self.tolerance.max_brightness = 255.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_less(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 16;
        self.tolerance.blue = 16;
        self.tolerance.green = 16;
        self.tolerance.red = 16;
        self.tolerance.min_brightness = 16.0;
        self.tolerance.max_brightness = 240.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_antialiasing(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 32;
        self.tolerance.blue = 32;
        self.tolerance.green = 32;
        self.tolerance.red = 32;
        self.tolerance.min_brightness = 64.0;
        self.tolerance.max_brightness = 96.0;
        self.ignore_antialiasing = true;
        self.ignore_colors = false;
        self
    }

    pub fn ignore_colors(mut self) -> Self {
        self.ignore_antialiasing = false;
        self.tolerance.alpha = 16;
        self.tolerance.min_brightness = 16.0;
        self.tolerance.max_brightness = 240.0;
        self.ignore_antialiasing = false;
        self.ignore_colors = true;
        self
    }
}

#[derive(Default)]
struct Tolerance {
    alpha: u8,
    max_brightness: f32,
    min_brightness: f32,
    red: u8,
    green: u8,
    blue: u8,
}

fn compare_pixel<I1, I2>(
    pixel1: &Rgba,
    pixel2: &Rgba,
    img1: &I1,
    img2: &I2,
    position: (u32, u32),
    opt: &ComparisonOptions,
) -> bool
where
    I1: GenericImage<Pixel = Rgba>,
    I2: GenericImage<Pixel = Rgba>,
{
    if !is_similar(
        pixel1.a() as i16,
        pixel2.a() as i16,
        opt.tolerance.alpha as i16,
    ) {
        false
    } else if opt.ignore_colors {
        is_pixel_brightness_similar(pixel1, pixel2, &opt.tolerance)
    } else if is_rgb_similar(pixel1, pixel2, &opt.tolerance) {
        true
    } else if opt.ignore_antialiasing
        && (is_antialiased(pixel1, img1, &position, &opt.tolerance)
            || is_antialiased(pixel2, img2, &position, &opt.tolerance))
    {
        is_pixel_brightness_similar(pixel1, pixel2, &opt.tolerance)
    } else {
        false
    }
}

fn abs_sub<T>(x: T, y: T) -> T
where
    T: PartialOrd + Signed,
{
    if x < y {
        y - x
    } else {
        x - y
    }
}

fn get_brightness(rgba: &Rgba) -> f32 {
    0.3 * rgba.r() as f32 + 0.59 * rgba.g() as f32 + 0.11 * rgba.b() as f32
}

fn get_hue(rgba: &Rgba) -> f32 {
    let (r, g, b) = (rgba.r() as f32, rgba.g() as f32, rgba.b() as f32);
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
where
    I: GenericImage<Pixel = image::Rgba<u8>>,
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
    let mut has_sibling_with_different_hue = 0;
    let mut has_high_contrast_sibling = 0;

    for x in left..right {
        for y in top..bottom {
            // ignore source pixel
            if x == p.0 && y == p.1 {
                continue;
            }

            let p2 = image.get_pixel(x, y);
            let brightness2 = get_brightness(&p2);
            let hue2 = get_hue(&p2);

            if abs_sub(brightness1, brightness2) > tolerance.max_brightness {
                has_high_contrast_sibling += 1;
            }

            if abs_sub(hue1, hue2) > 0.3 {
                has_sibling_with_different_hue += 1;
            }

            if is_rgb_same(&p1, &p2) {
                has_equivalent_sibling += 1;
            }

            if has_sibling_with_different_hue > 1 || has_high_contrast_sibling > 1 {
                return true;
            }
        }
    }

    has_equivalent_sibling < 2
}

fn is_pixel_brightness_similar(p1: &Rgba, p2: &Rgba, tolerance: &Tolerance) -> bool {
    let brightness1 = get_brightness(p1);
    let brightness2 = get_brightness(p2);
    is_similar(
        brightness1 as f32,
        brightness2 as f32,
        tolerance.min_brightness,
    )
}

fn is_rgb_same(p1: &Rgba, p2: &Rgba) -> bool {
    p1.r() == p2.r() && p1.g() == p2.g() && p1.b() == p2.b()
}

fn is_similar<T: Signed + std::cmp::PartialOrd>(v1: T, v2: T, tolerance: T) -> bool {
    abs_sub(v1, v2) <= tolerance
}

fn is_rgb_similar(p1: &Rgba, p2: &Rgba, t: &Tolerance) -> bool {
    is_similar(p1.r() as i16, p2.r() as i16, t.red as i16)
        && is_similar(p1.g() as i16, p2.g() as i16, t.green as i16)
        && is_similar(p1.b() as i16, p2.b() as i16, t.blue as i16)
}

fn width_height_from_2_images<I1, I2>(img1: &I1, img2: &I2) -> (u32, u32)
where
    I1: GenericImage<Pixel = Rgba> + 'static,
    I2: GenericImage<Pixel = Rgba> + 'static,
{
    let (width1, height1) = img1.dimensions();
    let (width2, height2) = img2.dimensions();

    let width = min(width1, width2);
    let height = min(height1, height2);

    (width, height)
}

fn xy_from_index(width: u32, index: u32) -> (u32, u32) {
    let y = index / width;
    let x = index - width * y;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn it_compare_image() {
        let img1 =
            &image::open(&Path::new("./examples/people1.jpg")).expect("unable to load people1.jpg");

        let img2 =
            &image::open(&Path::new("./examples/people2.jpg")).expect("unable to load people2.jpg");

        let opts = &ComparisonOptions::new();
        let r = compare_images(img1, img2, opts);

        assert_eq!(r.mismatch_percent, 97.1228);
    }
}

trait RgbaEx {
    fn r(&self) -> u8;
    fn g(&self) -> u8;
    fn b(&self) -> u8;
    fn a(&self) -> u8;
}

impl RgbaEx for Rgba {
    fn r(&self) -> u8 {
        self.channels()[0]
    }

    fn g(&self) -> u8 {
        self.channels()[1]
    }

    fn b(&self) -> u8 {
        self.channels()[2]
    }

    fn a(&self) -> u8 {
        self.channels()[3]
    }
}
