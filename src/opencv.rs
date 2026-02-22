use image::{GrayImage, Luma, RgbImage, Rgba, RgbaImage};

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

        if p1[0] != p2[0] || p1[1] != p2[1] || p1[2] != p2[2] {
            mask.put_pixel(x, y, Luma([255]));
        } else {
            mask.put_pixel(x, y, Luma([0]));
        }
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

        if p1[0] != p2[0] || p1[1] != p2[1] || p1[2] != p2[2] {
            mask.put_pixel(x, y, Luma([255]));
        } else {
            mask.put_pixel(x, y, Luma([0]));
        }
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

pub fn hsva_in_range(hsva_data: &[u8], lower: [u8; 3], upper: [u8; 3]) -> Vec<u8> {
    let mut mask = Vec::with_capacity(hsva_data.len() / 4);

    for chunk in hsva_data.chunks_exact(4) {
        let h = chunk[0];
        let s = chunk[1];
        let v = chunk[2];

        let is_in_range = h >= lower[0]
            && h <= upper[0]
            && s >= lower[1]
            && s <= upper[1]
            && v >= lower[2]
            && v <= upper[2];

        mask.push(if is_in_range { 255 } else { 0 });
    }

    mask
}
