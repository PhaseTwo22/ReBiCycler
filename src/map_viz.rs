use image::{ImageBuffer, Rgba, RgbaImage};
use rust_sc2::{geometry::Size, pixel_map::Pixel};

use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    pub fn map_siting(&mut self, frame_no: usize) {
        let mut image = self.background_map(255);

        for (point, bl) in self.siting_director.iter() {
            let contained_points = bl.size().contained_points(point);
            let color = bl.color(200);
            for (x, y) in contained_points {
                if point_within_image(&self.game_info.map_size, (x, y)) {
                    image.put_pixel(x, y, color);
                };
            }
        }

        if image
            .save(format!("replays/siting/{frame_no}.png"))
            .is_err()
        {
            self.unhandle_unhandle("Unable to save siting image to file.".to_string());
        };
    }

    pub fn background_map(&self, a: u8) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let grid = &self.game_info.pathing_grid;
        let mut image = RgbaImage::new(grid.dim().0 as u32, grid.dim().1 as u32);
        for (i, val) in grid.iter().enumerate() {
            let (x, y) = ((i / grid.dim().1) as u32, (i % grid.dim().0) as u32);
            let color = Rgba(match val {
                Pixel::Set => [0, 0, 0, a],
                Pixel::Empty => [50, 50, 50, a],
            });
            image.put_pixel(x, y, color);
        }
        image
    }
}

const fn point_within_image(image_size: &Size, point: (u32, u32)) -> bool {
    point.0 < image_size.x as u32 && point.1 < image_size.y as u32
}
