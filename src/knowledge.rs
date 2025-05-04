use std::collections::{HashMap, HashSet};

use rust_sc2::{
    game_state::Alert,
    ids::UnitTypeId,
    player::Race,
    prelude::{Alliance, Point2, UnitsIterator},
    unit::Unit,
    units::Units,
};

/// Stores things we know about the game.
#[derive(Default)]
pub struct Knowledge {
    pub confirmed_dead: HashMap<u64, UnitKnowledge>,
    pub first_seen_unit_times: HashMap<UnitTypeId, usize>,
    pub first_seen_friendly_times: HashMap<UnitTypeId, usize>,
    pub seen_units: HashMap<u64, UnitKnowledge>,
    pub confirmed_enemy_race: Option<Race>,
    pub total_spend: (u32, u32),
    pub total_reimbursed: (u32, u32),
    pub expansions_need_detectors: bool,
    pub expansions_need_clearing: bool,
}

/// a tidbit of information about a unit.
#[derive(Clone)]
pub struct UnitKnowledge {
    pub type_id: UnitTypeId,
    pub last_seen: usize,
    pub last_position: Point2,
    pub alliance: Alliance,
    pub is_structure: bool,
}
impl UnitKnowledge {
    /// generate relevant information from a unit for storage.
    fn from_unit(unit: &Unit, frame_no: usize) -> Self {
        Self {
            type_id: unit.type_id(),
            last_seen: frame_no,
            last_position: unit.position(),
            alliance: unit.alliance(),
            is_structure: unit.is_structure(),
        }
    }
}

impl crate::protoss_bot::ReBiCycler {
    /// Called by `on_step` to update our knowledge of the game state
    pub fn observe(&mut self, frame_no: usize) {
        self.state.action_errors.iter().for_each(|error| {
            println!("Action failed: {error:?}");
        });

        if self
            .state
            .observation
            .alerts
            .iter()
            .any(|a| matches!(&a, Alert::TransformationComplete))
        {
            let warpgates = self
                .units
                .my
                .structures
                .iter()
                .of_type(UnitTypeId::WarpGate)
                .map(rust_sc2::prelude::Unit::tag)
                .collect();
            self.siting_director.check_morph_gateways(warpgates);
        }

        // let effects = &self.state.observation.raw.effects;
        // if !effects.is_empty() {
        //     let ids: Vec<EffectId> = effects.iter().map(|e| e.id).collect();
        //     println!("Active effects: {ids:?}");
        // }

        let seen_units = self.units.all.clone();

        self.knowledge.update_seen_units(&seen_units, frame_no);

        self.knowledge.add_newly_seen_units(&seen_units, frame_no);
    }
}

impl Knowledge {
    /// when we detect a random player's race, store it
    pub fn confirm_race(&mut self, race: Race) {
        self.confirmed_enemy_race = Some(race);
    }

    /// We want to know when we first saw new enemy units. I think this will help us determine when we're being rushed, or benchmark our own build
    pub fn add_newly_seen_units(&mut self, units: &Units, frame_no: usize) {
        let new_units: HashSet<UnitTypeId> = units
            .iter()
            .filter_map(|u| {
                if self.first_seen_unit_times.contains_key(&u.type_id()) {
                    None
                } else {
                    Some(u.type_id())
                }
            })
            .collect();
        for unit_type in new_units {
            self.first_seen_unit_times.insert(unit_type, frame_no);
            println!("Newly seen: {unit_type:?}; frame {frame_no:?}");
        }
    }

    /// Store knowledge about every unit seen this frame
    pub fn update_seen_units(&mut self, seen_units: &Units, frame_no: usize) {
        for unit in seen_units {
            let new_knowledge = UnitKnowledge::from_unit(unit, frame_no);
            self.seen_units
                .entry(unit.tag())
                .insert_entry(new_knowledge);
        }
    }

    /// Store info for when a unit is destroyed
    pub fn unit_destroyed(&mut self, unit_tag: u64) -> Result<UnitKnowledge, KnowledgeError> {
        let unit = self
            .seen_units
            .remove(&unit_tag)
            .ok_or(KnowledgeError::UnknownUnitDestroyed)?;
        self.confirmed_dead.insert(unit_tag, unit.clone());
        Ok(unit)
    }
    /// fetch all the locations where we've seen enemy buildings
    pub fn get_enemy_buildings(&self) -> Vec<&UnitKnowledge> {
        self.seen_units
            .values()
            .filter(|uk| uk.is_structure && matches!(uk.alliance, Alliance::Enemy))
            .collect()
    }
}

#[derive(Debug)]
pub enum KnowledgeError {
    UnknownUnitDestroyed,
}
