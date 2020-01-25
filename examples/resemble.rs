extern crate image;
extern crate rust_resemble;

use rust_resemble::{compare_images, ComparisonOptions};
use std::path::Path;

fn main() {
    let img1 =
        image::open(&Path::new("./examples/people1.jpg")).expect("unable to load people1.jpg");

    let img2 =
        image::open(&Path::new("./examples/people2.jpg")).expect("unable to load people2.jpg");

    let opts = ComparisonOptions::new().ignore_less();
    let result = compare_images(&img1, &img2, &opts);

    println!("diff by {}%", result.mismatch_percent);

    result
        .image
        .save("./examples/diff.jpg")
        .expect("unable to save diff.jpg");
}
