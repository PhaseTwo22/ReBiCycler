use crate::protoss_bot::ReBiCycler;
use rust_sc2::{action::Target, prelude::Unit};
impl ReBiCycler {
    /// clears unit orders and sends it to every non-visible expansion location.
    pub fn send_on_expansion_scouting(&self, unit: &Unit) {
        unit.stop(false);
        let _: () = self
            .expansions
            .iter()
            .filter_map(|expo| {
                if !self.is_visible(expo.loc) {
                    unit.move_to(Target::Pos(expo.loc), true);
                    Some(())
                } else {
                    None
                }
            })
            .collect();
    }
}
