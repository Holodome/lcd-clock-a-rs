use image::io::Reader as ImageReader;
use std::{fs::File, io::Write, path::PathBuf};
use walkdir::WalkDir;

fn main() {
    let target_dir = PathBuf::from("target/img/");
    for entry in WalkDir::new("misc/img").into_iter().filter_map(|e| e.ok()) {
        let metadata = entry.metadata().unwrap();
        if !metadata.is_file() {
            continue;
        }
        let path = entry.path();
        if let Ok(image) = ImageReader::open(path).unwrap().decode() {
            let image = image.into_rgb8();
            let dim = image.dimensions();

            let img_raw = image.into_raw();
            let dim_raw = [dim.0.to_le_bytes(), dim.1.to_le_bytes()].concat();
            let mut target_name = target_dir.join(path);
            target_name.set_extension("bin");
            std::fs::create_dir_all(&target_name.parent().unwrap()).ok();
            let mut file = File::create(target_name).unwrap();
            file.write_all(&dim_raw).unwrap();
            file.write_all(&img_raw).unwrap();

            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }

    assert!(false);
}
