use image::io::Reader as ImageReader;
use std::{fs::File, io::Write, path::PathBuf};
use walkdir::WalkDir;

fn convert_rgb8_to_rgb565(src: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut dst = Vec::with_capacity(width * height * 2);
    for row in 0..height {
        for col in 0..width {
            let offset = (row * width + col) * 3;
            let r = src[offset] as u16;
            let g = src[offset + 1] as u16;
            let b = src[offset + 2] as u16;

            let b = b >> 3;
            let g = (g >> 2) << 5;
            let r = (r >> 3) << 11;

            let rgb = r | g | b;

            dst.push((rgb >> 8) as u8);
            dst.push((rgb & 0xFF) as u8);
        }
    }

    dst
}

fn main() {
    let target_dir = PathBuf::from("target/img/");
    let src_dir = "misc/img";
    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let metadata = entry.metadata().unwrap();
        if !metadata.is_file() {
            continue;
        }
        let path = entry.path();
        if let Ok(image) = ImageReader::open(path).unwrap().decode() {
            let image = image.into_rgb8();
            let dim = image.dimensions();

            let img_raw = image.into_raw();
            let img_raw = convert_rgb8_to_rgb565(&img_raw, dim.0 as usize, dim.1 as usize);

            let dim_raw = [dim.0.to_le_bytes(), dim.1.to_le_bytes()].concat();

            let path = path.strip_prefix(src_dir).unwrap();
            let mut target_name = target_dir.join(path);
            target_name.set_extension("bin");
            std::fs::create_dir_all(target_name.parent().unwrap()).ok();

            let mut file = File::create(target_name).unwrap();
            file.write_all(&dim_raw).unwrap();
            file.write_all(&img_raw).unwrap();

            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }
}
