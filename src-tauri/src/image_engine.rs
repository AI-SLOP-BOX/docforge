use image::{DynamicImage, RgbImage, Rgb, ImageBuffer, GenericImageView};
use image::imageops::FilterType;

pub fn process_scanned_images(
    paths: &[String],
    remove_shadow: bool,
    correct_perspective: bool,
    dpi: u32,
) -> Result<Vec<u8>, String> {
    let mut images: Vec<DynamicImage> = Vec::new();

    for path in paths {
        let img = image::open(path)
            .map_err(|e| format!("Failed to open {path}: {e}"))?;
        images.push(img);
    }

    let mut processed = Vec::new();

    for img in images {
        let mut result = img;

        if correct_perspective {
            result = correct_perspective_simple(&result);
        }

        if remove_shadow {
            result = remove_shadow_simple(&result);
        }

        let target_width = (result.width() as f64 * dpi as f64 / 150.0) as u32;
        let target_height = (result.height() as f64 * dpi as f64 / 150.0) as u32;
        result = result.resize(target_width, target_height, FilterType::Lanczos3);

        let mut rgb: RgbImage = result.to_rgb8();
        enhance_contrast(&mut rgb);

        let dynamic = DynamicImage::ImageRgb8(rgb);
        let mut buf = std::io::Cursor::new(Vec::new());
        dynamic.write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode: {e}"))?;
        processed.extend_from_slice(&buf.into_inner());
    }

    Ok(processed)
}

fn correct_perspective_simple(img: &DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgb = img.to_rgb8();

    let gray = to_grayscale(&rgb);
    let edges = sobel_edges(&gray, w, h);

    let corners = find_document_corners(&edges, w, h);

    if let Some((tl, tr, br, bl)) = corners {
        perspective_transform(img, tl, tr, br, bl)
    } else {
        img.clone()
    }
}

fn to_grayscale(rgb: &RgbImage) -> Vec<u8> {
    rgb.pixels().map(|p| {
        let r = p[0] as u32;
        let g = p[1] as u32;
        let b = p[2] as u32;
        ((r * 299 + g * 587 + b * 114) / 1000) as u8
    }).collect()
}

fn sobel_edges(gray: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut edges = vec![0u8; (w * h) as usize];

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = (y * w + x) as usize;
            let gx = -(gray[((y-1)*w + x-1) as usize] as i16)
                + gray[((y-1)*w + x+1) as usize] as i16
                - 2 * gray[(y*w + x-1) as usize] as i16
                + 2 * gray[(y*w + x+1) as usize] as i16
                - gray[((y+1)*w + x-1) as usize] as i16
                + gray[((y+1)*w + x+1) as usize] as i16;

            let gy = -(gray[((y-1)*w + x-1) as usize] as i16)
                - 2 * gray[((y-1)*w + x) as usize] as i16
                - gray[((y-1)*w + x+1) as usize] as i16
                + gray[((y+1)*w + x-1) as usize] as i16
                + 2 * gray[((y+1)*w + x) as usize] as i16
                + gray[((y+1)*w + x+1) as usize] as i16;

            let magnitude = ((gx * gx + gy * gy) as f64).sqrt() as u8;
            edges[idx] = if magnitude > 50 { 255 } else { 0 };
        }
    }
    edges
}

fn find_document_corners(edges: &[u8], w: u32, h: u32) -> Option<((f64, f64), (f64, f64), (f64, f64), (f64, f64))> {
    let margin_x = w / 10;
    let margin_y = h / 10;

    let mut top_edge = h;
    let mut bottom_edge = 0u32;
    let mut left_edge = w;
    let mut right_edge = 0u32;

    for y in margin_y..h - margin_y {
        for x in margin_x..w - margin_x {
            if edges[(y * w + x) as usize] > 0 {
                if y < top_edge { top_edge = y; }
                if y > bottom_edge { bottom_edge = y; }
                if x < left_edge { left_edge = x; }
                if x > right_edge { right_edge = x; }
            }
        }
    }

    if bottom_edge > top_edge + 10 && right_edge > left_edge + 10 {
        let tl = (left_edge as f64 + 5.0, top_edge as f64 + 5.0);
        let tr = (right_edge as f64 - 5.0, top_edge as f64 + 5.0);
        let br = (right_edge as f64 - 5.0, bottom_edge as f64 - 5.0);
        let bl = (left_edge as f64 + 5.0, bottom_edge as f64 - 5.0);
        Some((tl, tr, br, bl))
    } else {
        None
    }
}

fn perspective_transform(
    img: &DynamicImage,
    tl: (f64, f64),
    tr: (f64, f64),
    br: (f64, f64),
    bl: (f64, f64),
) -> DynamicImage {
    let src = img.to_rgb8();
    let (sw, sh) = src.dimensions();
    let dst_w = ((br.0 - bl.0).abs().max((tr.0 - tl.0).abs())) as u32;
    let dst_h = ((bl.1 - tl.1).abs().max((br.1 - tr.1).abs())) as u32;

    if dst_w == 0 || dst_h == 0 {
        return img.clone();
    }

    let mut dst: RgbImage = ImageBuffer::new(dst_w, dst_h);

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let fx = dx as f64 / dst_w as f64;
            let fy = dy as f64 / dst_h as f64;

            let sx = (tl.0 * (1.0 - fx) * (1.0 - fy) + tr.0 * fx * (1.0 - fy)
                + br.0 * fx * fy + bl.0 * (1.0 - fx) * fy) as u32;
            let sy = (tl.1 * (1.0 - fx) * (1.0 - fy) + tr.1 * fx * (1.0 - fy)
                + br.1 * fx * fy + bl.1 * (1.0 - fx) * fy) as u32;

            if sx < sw && sy < sh {
                let pixel = src.get_pixel(sx, sy);
                dst.put_pixel(dx, dy, *pixel);
            }
        }
    }

    DynamicImage::ImageRgb8(dst)
}

fn remove_shadow_simple(img: &DynamicImage) -> DynamicImage {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();

    let mut result = RgbImage::new(w, h);

    let window_size = 31;
    let half_win = window_size / 2;

    let mut integral = vec![0u64; ((w + 1) * (h + 1)) as usize];

    for y in 0..h {
        for x in 0..w {
            let p = rgb.get_pixel(x, y);
            let lum = (p[0] as u64 + p[1] as u64 + p[2] as u64) / 3;
            let idx = ((y + 1) * (w + 1) + x + 1) as usize;
            integral[idx] = lum
                + integral[((y) * (w + 1) + x + 1) as usize]
                + integral[((y + 1) * (w + 1) + x) as usize]
                - integral[((y) * (w + 1) + x) as usize];
        }
    }

    for y in 0..h {
        for x in 0..w {
            let x1 = x.saturating_sub(half_win).min(w - 1);
            let y1 = y.saturating_sub(half_win).min(h - 1);
            let x2 = (x + half_win).min(w - 1);
            let y2 = (y + half_win).min(h - 1);

            let count = ((x2 - x1 + 1) * (y2 - y1 + 1)) as u64;
            let sum = integral[((y2 + 1) * (w + 1) + x2 + 1) as usize]
                + integral[(y1 * (w + 1) + x1) as usize]
                - integral[((y2 + 1) * (w + 1) + x1) as usize]
                - integral[(y1 * (w + 1) + x2 + 1) as usize];

            let mean = sum / count;
            let p = rgb.get_pixel(x, y);

            let scale = if mean > 10 { 128.0 / mean as f64 } else { 1.0 };
            let scale = scale.clamp(0.5, 2.0);

            let nr = (p[0] as f64 * scale).min(255.0) as u8;
            let ng = (p[1] as f64 * scale).min(255.0) as u8;
            let nb = (p[2] as f64 * scale).min(255.0) as u8;

            result.put_pixel(x, y, Rgb([nr, ng, nb]));
        }
    }

    DynamicImage::ImageRgb8(result)
}

fn enhance_contrast(img: &mut RgbImage) {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        let lum = ((p[0] as u32 + p[1] as u32 + p[2] as u32) / 3) as usize;
        hist[lum] += 1;
    }

    let total = img.width() * img.height();
    let mut cumulative = [0u32; 256];
    cumulative[0] = hist[0];
    for i in 1..256 {
        cumulative[i] = cumulative[i - 1] + hist[i];
    }

    let lut: Vec<u8> = (0..256).map(|i| {
        ((cumulative[i] as f64 / total as f64) * 255.0).min(255.0) as u8
    }).collect();

    for p in img.pixels_mut() {
        p[0] = lut[p[0] as usize];
        p[1] = lut[p[1] as usize];
        p[2] = lut[p[2] as usize];
    }
}
