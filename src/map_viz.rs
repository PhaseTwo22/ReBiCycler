use image::{ImageBuffer, Pixel as ImagePixel, Rgba, RgbaImage};
use rust_sc2::{geometry::Size, pixel_map::Pixel as MapPixel};

use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    pub fn map_siting(&mut self, frame_no: usize) {
        let mut image = self.background_map(255);

        for (point, bl) in self.siting_director.iter() {
            let contained_points = bl.size().contained_points(*point);
            let color = bl.color(200);
            for (x, y) in contained_points {
                if point_within_image(&self.game_info.map_size, (x, y)) {
                    image.get_pixel_mut(x, y).blend(&color);
                };
            }
        }

        if image
            .save(format!("replays/siting/{frame_no}.png"))
            .is_err()
        {
            self.log_error("Unable to save siting image to file.".to_string());
        };
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn background_map(&self, a: u8) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let grid = &self.game_info.pathing_grid;
        let mut image = RgbaImage::new(grid.dim().0 as u32, grid.dim().1 as u32);
        for (i, val) in grid.iter().enumerate() {
            let (x, y) = ((i / grid.dim().1) as u32, (i % grid.dim().0) as u32);
            let color = Rgba(match val {
                MapPixel::Set => [0, 0, 0, a],
                MapPixel::Empty => [50, 50, 50, a],
            });
            image.put_pixel(x, y, color);
        }
        image
    }
}
#[allow(clippy::cast_possible_truncation)]
const fn point_within_image(image_size: &Size, point: (u32, u32)) -> bool {
    point.0 < image_size.x as u32 && point.1 < image_size.y as u32
}
