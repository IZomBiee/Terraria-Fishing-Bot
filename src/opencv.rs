pub fn rgba_2_hsva(raw_data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw_data.len());

    for chunk in raw_data.chunks_exact(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

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

        output.push((h * 180.0) as u8);
        output.push((s * 255.0) as u8);
        output.push((v * 255.0) as u8);
        output.push(chunk[3]);
    }
    output
}

pub fn rgba_difference_mask(img1: &[u8], img2: &[u8]) -> Vec<u8> {
    assert_eq!(
        img1.len(),
        img2.len(),
        "Images must have the same dimensions"
    );

    let mut mask = Vec::with_capacity(img1.len() / 4);

    for (px1, px2) in img1.chunks_exact(4).zip(img2.chunks_exact(4)) {
        let r1 = px1[0];
        let g1 = px1[1];
        let b1 = px1[2];

        let r2 = px2[0];
        let g2 = px2[1];
        let b2 = px2[2];

        if r1 != r2 || g1 != g2 || b1 != b2 {
            mask.push(255);
        } else {
            mask.push(0);
        }
    }

    mask
}

pub fn circle(
    rgba_data: &mut [u8],
    width: u32,
    height: u32,
    center_x: u32,
    center_y: u32,
    radius: i32,
) {
    let center_x = center_x as i32;
    let center_y = center_y as i32;
    let color = [0, 255, 127, 255];

    let x_start = (center_x - radius).max(0);
    let x_end = (center_x + radius).min(width as i32 - 1);
    let y_start = (center_y - radius).max(0);
    let y_end = (center_y + radius).min(height as i32 - 1);

    for y in y_start..=y_end {
        for x in x_start..=x_end {
            let dx = x - center_x;
            let dy = y - center_y;

            if dx * dx + dy * dy <= radius * radius {
                let offset = ((y * width as i32 + x) * 4) as usize;

                if offset + 3 < rgba_data.len() {
                    rgba_data[offset..offset + 4].copy_from_slice(&color);
                }
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

#[derive(Debug, Clone, Copy)]
pub struct Blob {
    pub center_x: u32,
    pub center_y: u32,
    pub area: u32,
}

pub fn find_blobs(mask: &mut [u8], width: u32, height: u32, min_blob_size: u32) -> Vec<Blob> {
    let mut blobs = Vec::new();
    let w = width as i32;
    let h = height as i32;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;

            if mask[idx] == 255 {
                let mut sum_x = 0i64;
                let mut sum_y = 0i64;
                let mut pixel_count = 0u32;

                let mut stack = vec![(x, y)];
                mask[idx] = 0;

                while let Some((curr_x, curr_y)) = stack.pop() {
                    sum_x += curr_x as i64;
                    sum_y += curr_y as i64;
                    pixel_count += 1;

                    for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                        let nx = curr_x + dx;
                        let ny = curr_y + dy;

                        if nx >= 0 && nx < w && ny >= 0 && ny < h {
                            let n_idx = (ny * w + nx) as usize;
                            if mask[n_idx] == 255 {
                                mask[n_idx] = 0;
                                stack.push((nx, ny));
                            }
                        }
                    }
                }

                if pixel_count > min_blob_size {
                    blobs.push(Blob {
                        center_x: (sum_x / pixel_count as i64) as u32,
                        center_y: (sum_y / pixel_count as i64) as u32,
                        area: pixel_count,
                    });
                }
            }
        }
    }
    blobs
}
