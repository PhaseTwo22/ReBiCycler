use rust-sc2::prelude::Unit;
use crate::protoss_bot::ReBiCycler;
impl ReBiCycler {

    /// clears unit orders and sends it to every non-visible expansion location. 
    pub fn send_on_expansion_scouting(&self, unit: &Unit {
        unit.stop(false);
        self.expansions.iter().filter_map(|expo| if !self.is_visible(expo.location) {Some(unit.move_to(expo.location)} else {None}).collect();
        
    }

}