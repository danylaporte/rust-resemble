# rust-resemble

Analyse and compare images.

![Example](readmeimage.jpg)


## Example


```rust
extern crate rust_resemble;
extern crate image;

use std::fs::File;
use std::path::Path;
use image::ImageFormat;
use rust_resemble::{compare_images, ComparisonOptions};

fn main() {
    // load the source images from disk
    let img1 = image::open(&Path::new("./examples/people1.jpg")).unwrap();
    let img2 = image::open(&Path::new("./examples/people2.jpg")).unwrap();

    // configure the comparison
    let opts = ComparisonOptions::new().ignore_less();

    // compare the images
    let result = compare_images(&img1, &img2, &opts);
    println!("diff by {}%", result.mismatch_percent);

    // save the diff image to disk
    let mut f = File::create("./examples/diff.jpg").unwrap();

    result.image.save(&mut f, ImageFormat::JPEG).unwrap();
}

```

## Credits

Based on [resemble.js](https://github.com/Huddle/Resemble.js/)