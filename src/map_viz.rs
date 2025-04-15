use image::{ImageBuffer, RgbaImage};

use crate::{
    protoss_bot::ReBiCycler,
    siting::{SitingDirector, SlotSize},
};

impl ReBiCycler {
    fn map_siting(&self) {
        let mut coords: Vec<((usize, usize), &str)> = Vec::new();

        for (point, bl) in self.siting_director.iter() {
            let contained_points = bl.size().contained_points(point);
            let color = bl.color();
            coords.append(&mut contained_points.map(|p| (p, color)).collect());
        }

        let mut image = RgbaImage::new(
            self.game_info.map_size.x as u32,
            self.game_info.map_size.y as u32,
        );
    }
}
