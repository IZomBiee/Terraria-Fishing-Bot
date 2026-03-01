use image::{GrayImage, ImageBuffer, Luma, Pixel, RgbImage, Rgba, RgbaImage};

pub fn rgba_2_hsva(img: &RgbaImage) -> RgbaImage {
    let (width, height) = img.dimensions();
    let mut hsva_img = RgbaImage::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let a = pixel[3];

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };
        let mut h = if delta == 0.0 {
            0.0
        } else if max == r {
            (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        h /= 6.0;

        let h_u8 = (h * 180.0) as u8;
        let s_u8 = (s * 255.0) as u8;
        let v_u8 = (v * 255.0) as u8;

        hsva_img.put_pixel(x, y, Rgba([h_u8, s_u8, v_u8, a]));
    }

    hsva_img
}

pub fn rgba_2_rgb(rgba: &RgbaImage) -> RgbImage {
    let (width, height) = rgba.dimensions();
    let mut rgb = RgbImage::new(width, height);

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let channels = pixel.channels();
        rgb.put_pixel(x, y, image::Rgb([channels[0], channels[1], channels[2]]));
    }

    rgb
}

pub fn rgba_difference_mask(img1: &RgbaImage, img2: &RgbaImage) -> GrayImage {
    assert_eq!(
        img1.dimensions(),
        img2.dimensions(),
        "Images must have the same dimensions"
    );

    let (width, height) = img1.dimensions();

    let mut mask = GrayImage::new(width, height);

    for (x, y, p1) in img1.enumerate_pixels() {
        let p2 = img2.get_pixel(x, y);

        let r_diff = (p1[0] as f32 - p2[0] as f32).powi(2);
        let g_diff = (p1[1] as f32 - p2[1] as f32).powi(2);
        let b_diff = (p1[2] as f32 - p2[2] as f32).powi(2);

        let distance = (r_diff + g_diff + b_diff).sqrt();
        let result = distance.min(255.0) as u8;

        mask.put_pixel(x, y, Luma([result]));
    }

    mask
}

pub fn rgb_difference_mask(img1: &RgbImage, img2: &RgbImage) -> GrayImage {
    assert_eq!(
        img1.dimensions(),
        img2.dimensions(),
        "Images must have the same dimensions"
    );

    let (width, height) = img1.dimensions();
    let mut mask = GrayImage::new(width, height);

    for (x, y, p1) in img1.enumerate_pixels() {
        let p2 = img2.get_pixel(x, y);

        let r_diff = (p1[0] as f32 - p2[0] as f32).powi(2);
        let g_diff = (p1[1] as f32 - p2[1] as f32).powi(2);
        let b_diff = (p1[2] as f32 - p2[2] as f32).powi(2);

        let distance = (r_diff + g_diff + b_diff).sqrt();
        let result = distance.min(255.0) as u8;

        mask.put_pixel(x, y, Luma([result]));
    }

    mask
}

pub fn circle(img: &mut RgbaImage, center_x: u32, center_y: u32, radius: i32) {
    let cx = center_x as i32;
    let cy = center_y as i32;
    let color = Rgba([0, 255, 127, 255]);

    let (width, height) = img.dimensions();

    let x_start = (cx - radius).max(0);
    let x_end = (cx + radius).min(width as i32 - 1);
    let y_start = (cy - radius).max(0);
    let y_end = (cy + radius).min(height as i32 - 1);

    for y in y_start..=y_end {
        for x in x_start..=x_end {
            let dx = x - cx;
            let dy = y - cy;

            if dx * dx + dy * dy <= radius * radius {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

pub fn rgba_in_range(frame: &RgbaImage, lower: [u8; 4], upper: [u8; 4]) -> GrayImage {
    let (width, height) = frame.dimensions();
    let raw_data = frame.as_raw();

    let mut mask_pixels = Vec::with_capacity((width * height) as usize);

    for chunk in raw_data.chunks_exact(4) {
        let is_in_range = chunk[0] >= lower[0]
            && chunk[0] <= upper[0]
            && chunk[1] >= lower[1]
            && chunk[1] <= upper[1]
            && chunk[2] >= lower[2]
            && chunk[2] <= upper[2]
            && chunk[3] >= lower[3]
            && chunk[3] <= upper[3];

        mask_pixels.push(if is_in_range { 255 } else { 0 });
    }

    ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(width, height, mask_pixels)
        .expect("Buffer size should match dimensions")
}
