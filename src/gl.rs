use crate::{
    drivers::st7789vwx6::Display, hardware::ST7789VWx6Ty, images::Image, lcd_clock::Error,
    misc::ColorRGB565,
};

/// Helper structure containing functions for drawing on displays. (Thus the
/// name - graphics library).
pub struct Gl<'a> {
    displays: &'a mut ST7789VWx6Ty,
}

impl<'a> Gl<'a> {
    pub fn new(displays: &'a mut ST7789VWx6Ty) -> Self {
        Self { displays }
    }

    pub fn fill(&mut self, display: Display, color: ColorRGB565) -> Result<(), Error> {
        let w = self.displays.width();
        let h = self.displays.height();
        self.displays
            .set_pixels_iter(
                display,
                0,
                0,
                w,
                h,
                (0..(w * h)).flat_map(|_| color.to_be()),
            )
            .map_err(Error::Display)
    }

    pub fn clear_all(&mut self, color: ColorRGB565) -> Result<(), Error> {
        for display in Display::all() {
            self.fill(display, color)?;
        }

        Ok(())
    }

    pub fn draw_rect(
        &mut self,
        display: Display,
        x_min: u16,
        y_min: u16,
        x_max: u16,
        y_max: u16,
        color: ColorRGB565,
    ) -> Result<(), Error> {
        self.displays
            .set_pixels_iter(
                display,
                x_min,
                y_min,
                x_max,
                y_max,
                (0..((x_max - x_min) * (y_max - y_min))).flat_map(|_| color.to_be()),
            )
            .map_err(Error::Display)
    }

    pub fn draw_pic(&mut self, display: Display, pic: &Image) -> Result<(), Error> {
        let w = pic.width() as u16;
        let h = pic.height() as u16;
        let pix = pic.pixels();
        self.displays
            .set_pixels(display, 0, 0, w, h, pix)
            .map_err(Error::Display)
    }

    pub fn draw_bounding_rect(
        &mut self,
        display: Display,
        thickness: usize,
        color: ColorRGB565,
    ) -> Result<(), Error> {
        let thickness = thickness as u16;
        let w = self.displays.width();
        let h = self.displays.height();
        self.draw_rect(display, 0, 0, w, thickness, color)?;
        self.draw_rect(display, 0, thickness, thickness, h, color)?;
        self.draw_rect(display, w - thickness, thickness, w, h, color)?;
        self.draw_rect(display, thickness, h - thickness, w - thickness, h, color)
    }
}
