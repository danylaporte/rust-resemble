# rust-resemble

Analyse and compare images.

![Example](readmeimage.jpg)


## Example


```rust
use image::open;
use rust_resemble::{compare_images, ComparisonOptions};
use std::path::Path;

fn main() {
    let img1 = open(&Path::new("./examples/people1.jpg")).expect("unable to load people1.jpg");
    let img2 = open(&Path::new("./examples/people2.jpg")).expect("unable to load people2.jpg");

    let opts = ComparisonOptions::new().ignore_less();
    let result = compare_images(&img1, &img2, &opts);

    println!("diff by {}%", result.mismatch_percent);

    result
        .image
        .save("./examples/diff.jpg")
        .expect("unable to save diff.jpg");
}
```

## Credits

Based on [resemble.js](https://github.com/Huddle/Resemble.js/)