//! Images used for displaying time and menu options on LCD's.
//! These are embedded directly in executable using include_bytes!.
//! Images are generated using build script (build.rs).

use crate::lcd_clock::AppMode;

pub struct Image(&'static [u8]);

impl Image {
    pub fn width(&self) -> u32 {
        u32::from_le_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }

    pub fn height(&self) -> u32 {
        u32::from_le_bytes([self.0[4], self.0[5], self.0[6], self.0[7]])
    }

    pub fn pixels(&self) -> &[u8] {
        &self.0[8..]
    }
}

const fn make_image(data: &'static [u8]) -> Image {
    Image(data)
}

pub struct Numpic([Image; 10]);

impl Numpic {
    pub fn get_digit(&self, digit: u8) -> Option<&Image> {
        let digit = digit as usize;
        if digit < self.0.len() {
            Some(&self.0[digit])
        } else {
            None
        }
    }
}

macro_rules! make_numpic_img {
    ($letter:literal, $num:literal) => {
        make_image(include_bytes!(concat!(
            "../target/img/numpic/",
            $letter,
            "/",
            $num,
            ".bin"
        )))
    };
}

macro_rules! make_numpic {
    ($name:ident, $letter:literal) => {
        pub const $name: Numpic = Numpic([
            make_numpic_img!($letter, 0),
            make_numpic_img!($letter, 1),
            make_numpic_img!($letter, 2),
            make_numpic_img!($letter, 3),
            make_numpic_img!($letter, 4),
            make_numpic_img!($letter, 5),
            make_numpic_img!($letter, 6),
            make_numpic_img!($letter, 7),
            make_numpic_img!($letter, 8),
            make_numpic_img!($letter, 9),
        ]);
    };
}

make_numpic!(NUMPIC_A, "A");
make_numpic!(NUMPIC_B, "B");
make_numpic!(NUMPIC_C, "C");
make_numpic!(NUMPIC_D, "D");

pub struct Menupic([Image; 6]);

impl Menupic {
    pub fn get_pic(&self, opt: AppMode) -> &Image {
        match opt {
            AppMode::Time => &self.0[0],
            AppMode::Alarm => &self.0[1],
            AppMode::Rgb => &self.0[2],
            AppMode::Brightness => &self.0[3],
            AppMode::TempHumidity => &self.0[4],
            AppMode::Return => &self.0[5],
        }
    }
}

macro_rules! make_menupic_img {
    ($letter:literal, $num:literal) => {
        make_image(include_bytes!(concat!(
            "../target/img/menupic/",
            $letter,
            "/",
            $num,
            ".bin"
        )))
    };
}

macro_rules! make_menupic {
    ($name:ident, $letter:literal) => {
        pub const $name: Menupic = Menupic([
            make_menupic_img!($letter, 1),
            make_menupic_img!($letter, 2),
            make_menupic_img!($letter, 3),
            make_menupic_img!($letter, 4),
            make_menupic_img!($letter, 5),
            make_menupic_img!($letter, 6),
        ]);
    };
}

make_menupic!(MENUPIC_A, "A");
make_menupic!(MENUPIC_B, "B");
