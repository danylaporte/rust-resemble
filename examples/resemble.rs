extern crate rust_resemble;
extern crate image;

use std::fs::File;
use std::path::Path;
use image::ImageFormat;
use rust_resemble::{compare_images, ComparisonOptions};

fn main() {
    let img1 = image::open(&Path::new("./examples/people1.jpg"))
        .expect("unable to load people1.jpg");
    let img2 = image::open(&Path::new("./examples/people2.jpg"))
        .expect("unable to load people2.jpg");
    let opts = ComparisonOptions::new().ignore_less();

    let result = compare_images(&img1, &img2, &opts);
    println!("diff by {}%", result.mismatch_percent);

    let mut f = File::create("./examples/diff.jpg").expect("unable to save diff.jpg");

    result.image.save(&mut f, ImageFormat::JPEG).unwrap();
}