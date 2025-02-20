use base_manager::BaseManager;
use build_order::*;
use rust_sc2::prelude::*;
use siting::*;
use std::{any::Any, fmt::Debug};

mod base_manager;
mod build_order;
mod siting;

const CHRONOBOOST_COST: u32 = 50;

#[derive(Debug, PartialEq)]
pub struct Tag {
    tag: u64,
    type_id: UnitTypeId,
}
impl Tag {
    pub fn from_unit(unit: &Unit) -> Self {
        Tag {
            tag: unit.tag(),
            type_id: unit.type_id(),
        }
    }

    pub fn default() -> Tag {
        Tag { tag: 0, type_id: UnitTypeId::NotAUnit }
    }
}

#[bot]
#[derive(Default)]
pub struct ReBiCycler{
    bom: BuildOrderManager,
    base_managers: Vec<BaseManager>,
}
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        self.bom = BuildOrderManager::new();

        for worker in self.units.my.workers.clone().iter() {
            self.reassign_worker_to_nearest_base(worker)
        }

        println!("Game start!");
        println!(
            "Main Nexus has {:?} workers assigned.",
            self.base_managers.first().unwrap().workers().len()
        );
        Ok(())
    }

    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        self.observe();
        self.step_build();
        //self.micro();
        //println!("Step step step {}", frame_no);
        Ok(())
    }

    fn on_event(&mut self, event: Event) -> SC2Result<()> {
        match event {
            Event::ConstructionComplete(building_tag) => {
                let building = self
                    .units
                    .my
                    .structures
                    .iter()
                    .find_tags(&vec![building_tag])
                    .next()
                    .unwrap();
                println!(
                    "Building Finished! {:?}, {building_tag}",
                    building.type_id()
                );
                if building.type_id() == UnitTypeId::Nexus {
                    self.new_base_finished(building_tag);
                }
            }
            Event::UnitCreated(unit_tag) => {
                let unit = self.units.my.units.get(unit_tag).unwrap().clone();
                println!("New Unit! {:?}, {}", unit.type_id(), unit_tag);
                if unit.type_id() == UnitTypeId::Probe {
                    self.reassign_worker_to_nearest_base(&unit);
                }
            }
            Event::UnitDestroyed(unit_tag, alliance) => {
                if let Some(unit) = self.units.all.iter().find_tags(&vec![unit_tag]).next() {
                    println!(
                        "Unit destroyed! {:?}, {}, {:?}",
                        unit.type_id(),
                        unit_tag,
                        alliance
                    );
                }
            }
            Event::ConstructionStarted(building_tag) => {
                let building = self
                    .units
                    .my
                    .structures
                    .iter()
                    .find_tags(&vec![building_tag])
                    .next()
                    .unwrap();
                println!("New Building! {:?}, {building_tag}", building.type_id());
            }
            Event::RandomRaceDetected(race) => {
                println!("This cheeser is {:?}!", race);
            }
        }
        Ok(())
    }
}

impl ReBiCycler {
    pub fn new() -> Self {
        Self {
            /* initializing fields */
            bom: BuildOrderManager::new(),
            ..Default::default()
        }
    }

    pub fn new_base_finished(&mut self, base_tag: u64) {
        let bm = BaseManager::new(Tag { tag: base_tag, type_id: UnitTypeId::Nexus});
        self.base_managers.push(bm)
    }

    pub fn reassign_worker_to_nearest_base(&mut self, worker: &Unit) {
        if let Some(nexus) = self.units.my.townhalls.closest(worker.position()) {
            if let Some(closest_base) = self
                .base_managers
                .iter()
                .filter(|bm| {
                    &bm.nexus.unwrap_or(Tag::default()) == &Tag::from_unit(nexus)
                })
                .next()
            {
                closest_base.assign_unit(Tag::from_unit(worker));
            }
        }
    }
    fn observe(&mut self) {
        self.state.action_errors.iter().for_each(|error| {
            println!("Action failed: {:?}", error);
        });
    }

    fn step_build(&mut self) {
        self.progress_build();
        self.check_policies();
    }

    fn progress_build(&mut self) {
        if let Some(next_task) = self.bom.get_next_component() {
            if self.evaluate_condition(&next_task.prereq) {
                self.attempt_build_action(&next_task.action);
                self.bom.mark_component_done();
            }
        }
    }

    fn check_policies(&self) {
        for policy in &self.bom.policies {
            if !policy.active {
                continue;
            }
            if self.evaluate_condition(&policy.condition) {
                self.attempt_build_action(&policy.action);
            }
        }
    }

    fn evaluate_condition(&self, condition: &BuildCondition) -> bool {
        match condition {
            BuildCondition::Supply(supply) => self.supply_used < *supply,
            BuildCondition::Count(unit_type, count) => {
                let unit_count = self.counter().count(*unit_type);
                unit_count < *count
            }
            BuildCondition::SupplyLeft(remaining_supply) => self.supply_left < *remaining_supply,
            BuildCondition::Structure(structure_type) => self
                .units
                .my
                .structures
                .iter()
                .any(|u| u.type_id() == *structure_type),
            BuildCondition::Tech(upgrade) => self.upgrade_progress(*upgrade) > 0.95,
        }
    }

    fn attempt_build_action(&self, action: &BuildOrderAction) {
        match action {
            BuildOrderAction::Construct(unit_type) => {
                self.build(unit_type, self.start_center.towards(self.enemy_start, 2.0))
            }
            BuildOrderAction::Train(unit_type) => self.train(unit_type.clone()),
            BuildOrderAction::Chrono(unit_type) => self.chrono_boost(unit_type.clone()),
            BuildOrderAction::Research(upgrade, researcher, ability) => {
                self.research(researcher.clone(), upgrade.clone(), ability.clone())
            }
        }
    }

    fn build(&self, structure_type: &UnitTypeId, position: Point2) {
        let builder = self.units.my.workers.first().unwrap();
        builder.build(structure_type.clone(), position, false);
    }

    fn train(&self, unit_type: UnitTypeId) {
        let trainer = self.units.my.townhalls.first().unwrap();
        trainer.train(unit_type, false);
    }

    fn chrono_boost(&self, structure_type: UnitTypeId) {
        let mut energetic_nexi = self
            .units
            .my
            .townhalls
            .iter()
            .filter(|unit| unit.energy().unwrap_or(0) >= CHRONOBOOST_COST);
        let mut target = self
            .units
            .my
            .structures
            .iter()
            .of_type(structure_type)
            .unused()
            .next();
        if let (Some(nexus), Some(target)) = (energetic_nexi.next(), target) {
            nexus.command(
                AbilityId::EffectChronoBoost,
                Target::Tag(target.tag()),
                false,
            );
        }
    }

    pub fn research(&self, researcher: UnitTypeId, upgrade: UpgradeId, ability: AbilityId) {
        let researchers = self.units.my.all.filter(|unit| {
            unit.type_id() == researcher && unit.is_ready() && unit.orders().len() < 5
        });
        if researchers.len() > 0 && !self.has_upgrade(upgrade) && !self.is_ordered_upgrade(upgrade)
        {
            if let Some(candidate) = researchers.min(|unit| unit.orders().len()) {
                if self.can_afford_upgrade(upgrade) {
                    candidate.use_ability(ability, true);
                }
            }
        }
    }
}

pub struct UnitEmploymentError(String);
impl Debug for UnitEmploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in employment: {}", self.0)
    }
}
