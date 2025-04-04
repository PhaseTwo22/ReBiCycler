use std::collections::{HashMap, HashSet};

use rust_sc2::{
    ids::{EffectId, UnitTypeId},
    prelude::{Alliance, Point2},
    unit::Unit,
    units::Units,
};

#[derive(Default)]
pub struct Knowledge {
    pub confirmed_dead: HashMap<u64, UnitKnowledge>,
    pub first_seen_enemy_times: HashMap<UnitTypeId, usize>,
    pub seen_units: HashMap<u64, UnitKnowledge>,
}

#[derive(Clone)]
pub struct UnitKnowledge {
    pub type_id: UnitTypeId,
    pub last_seen: usize,
    pub last_position: Point2,
    pub alliance: Alliance,
}
impl UnitKnowledge {
    fn from_unit(unit: &Unit, frame_no: usize) -> Self {
        Self {
            type_id: unit.type_id(),
            last_seen: frame_no,
            last_position: unit.position(),
            alliance: unit.alliance(),
        }
    }
}

impl crate::protoss_bot::ReBiCycler {
    pub fn observe(&mut self, frame_no: usize) {
        self.state.action_errors.iter().for_each(|error| {
            println!("Action failed: {error:?}");
        });

        let effects = &self.state.observation.raw.effects;
        if !effects.is_empty() {
            let ids: Vec<EffectId> = effects.iter().map(|e| e.id).collect();
            println!("Active effects: {ids:?}");
        }

        let seen_units = self.units.all.clone();

        self.knowledge.update_seen_units(&seen_units, frame_no);

        self.knowledge.add_newly_seen_foes(&seen_units, frame_no);
    }
}

impl Knowledge {
    pub fn add_newly_seen_foes(&mut self, enemies: &Units, frame_no: usize) {
        let new_foes: HashSet<UnitTypeId> = enemies
            .iter()
            .filter_map(|u| {
                if u.is_enemy() && !self.first_seen_enemy_times.contains_key(&u.type_id()) {
                    Some(u.type_id())
                } else {
                    None
                }
            })
            .collect();
        for unit_type in new_foes {
            self.first_seen_enemy_times.insert(unit_type, frame_no);
        }
    }

    pub fn update_seen_units(&mut self, seen_units: &Units, frame_no: usize) {
        for unit in seen_units {
            let new_knowledge = UnitKnowledge::from_unit(unit, frame_no);
            self.seen_units
                .entry(unit.tag())
                .insert_entry(new_knowledge);
        }
    }

    pub fn unit_destroyed(&mut self, unit_tag: u64) -> Result<UnitKnowledge, KnowledgeError> {
        let unit = self
            .seen_units
            .remove(&unit_tag)
            .ok_or(KnowledgeError::UnknownUnitDestroyed)?;
        self.confirmed_dead.insert(unit_tag, unit.clone());
        Ok(unit)
    }
}

#[derive(Debug)]
pub enum KnowledgeError {
    UnknownUnitDestroyed,
}
