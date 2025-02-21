use base_manager::BaseManager;
use build_order::{BuildCondition, BuildOrderAction, BuildOrderManager};
use errors::UnitEmploymentError;
use rust_sc2::prelude::*;
use std::fmt::Debug;

mod base_manager;
mod build_order;
mod errors;
mod siting;

const CHRONOBOOST_COST: u32 = 50;

#[must_use]
pub fn get_options<'a>() -> LaunchOptions<'a> {
    LaunchOptions::<'a> {
        realtime: false,
        save_replay_as: Some("/home/andrew/Rust/ReBiCycler/replays/test"),
        ..Default::default()
    }
}

#[must_use]
pub fn distance_squared(a: &Point2, b: &Point2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx.mul_add(dx, dy * dy)
}

#[must_use]
pub fn closest_index(target: Point2, population: Vec<Point2>) -> Option<usize> {
    population
        .iter()
        .map(|pop| distance_squared(&target, pop))
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tag {
    tag: u64,
    type_id: UnitTypeId,
}
impl Tag {
    #[must_use]
    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            tag: unit.tag(),
            type_id: unit.type_id(),
        }
    }

    #[must_use]
    pub const fn default() -> Self {
        Self {
            tag: 0,
            type_id: UnitTypeId::NotAUnit,
        }
    }
}

#[bot]
#[derive(Default)]
pub struct ReBiCycler {
    bom: BuildOrderManager,
    base_managers: Vec<BaseManager>,
    game_started: bool,
}
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        self.bom = BuildOrderManager::new();

        for worker in &self.units.my.workers.clone() {
            self.reassign_worker_to_nearest_base(worker)
                .expect("No bases at game start?!");
        }

        println!("Game start!");
        self.game_started = true;
        Ok(())
    }

    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        self.observe();

        //self.micro();
        if frame_no % 100 == 0 {
            println!(
                "Step step step {}, M:{}, G:{}, S:{}/{}",
                frame_no, self.minerals, self.vespene, self.supply_used, self.supply_cap
            );
            self.step_build();
        };
        if frame_no >= 2000 && frame_no % 100 == 0 {
            if let Some(structure) = self.units.my.structures.first() {
                let _: () = self
                    .units
                    .my
                    .workers
                    .iter()
                    .map(|w| w.attack(Target::Tag(structure.tag()), false))
                    .collect();
            }
        }
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
                    self.new_base_finished(Tag::from_unit(building), building.position());
                }
            }
            Event::UnitCreated(unit_tag) => {
                let unit = self.units.my.units.get(unit_tag).unwrap().clone();
                println!("New Unit! {:?}, {}", unit.type_id(), unit_tag);
                if unit.type_id() == UnitTypeId::Probe && self.game_started {
                    self.reassign_worker_to_nearest_base(&unit);
                }
            }
            Event::UnitDestroyed(unit_tag, alliance) => {
                let unit = self.units.all.get(unit_tag);
                match unit {
                    Some(unit) => {
                        println!(
                            "Unit destroyed! {:?}, {}, {:?}",
                            unit.type_id(),
                            unit_tag,
                            alliance
                        );
                        let unit_tag = Tag::from_unit(unit);
                        if unit.is_structure() && unit.is_mine() {
                            self.base_managers
                                .iter_mut()
                                .any(|bm| bm.destroy_building_by_tag(unit_tag.clone()));
                        };
                    }
                    None => println!("Unknown unit destroyed: {unit_tag:?}"),
                };
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
                if self.enemy_race.is_random() {
                    println!("This cheeser is {race:?}!");
                };
            }
        }
        Ok(())
    }

    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        Ok(())
    }
}

impl ReBiCycler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            /* initializing fields */
            bom: BuildOrderManager::new(),
            game_started: false,
            ..Default::default()
        }
    }

    pub fn new_base_finished(&mut self, base: Tag, position: Point2) {
        let mut bm = BaseManager::new(
            Some(base),
            format!("Expansion {}", self.counter().count(UnitTypeId::Nexus)),
            position,
        );

        for resource in self.units.resources.iter().closer(10.0, position) {
            bm.assign_unit(resource);
        }

        for building in self.units.my.structures.iter().closer(15.0, position) {
            bm.add_building(building);
        }

        bm.siting_manager
            .add_pylon_site(position.towards(self.game_info.map_center, 10.0));

        self.base_managers.push(bm);
    }

    pub fn reassign_worker_to_nearest_base(
        &mut self,
        worker: &Unit,
    ) -> Result<(), UnitEmploymentError> {
        let nearest_nexus = self.units.my.townhalls.iter().closest(worker);
        if let Some(nn) = nearest_nexus {
            let nn_tag = Tag::from_unit(nn);
            self.base_managers
                .iter_mut()
                .find(|bm| bm.nexus == Some(nn_tag.clone()))
                .map_or(
                    Err(UnitEmploymentError("No base managers exist!".to_string())),
                    |bm| bm.assign_unit(worker),
                )
        } else {
            Err(UnitEmploymentError("No nexi exist!".to_string()))
        }
    }

    pub fn get_closest_base_manager(&mut self, position: Point2) -> Option<&mut BaseManager> {
        if self.base_managers.is_empty() {
            return None;
        }
        let bm_points = self.base_managers.iter().map(|bm| bm.location).collect();
        let nearest_bm = closest_index(position, bm_points);
        match nearest_bm {
            Some(index) => Some(&mut self.base_managers[index]),
            None => None,
        }
    }

    fn observe(&mut self) {
        self.state.action_errors.iter().for_each(|error| {
            println!("Action failed: {error:?}");
        });
    }

    fn step_build(&mut self) {
        self.progress_build();
        self.check_policies();
    }

    fn progress_build(&mut self) {
        if let Some(next_task) = self.bom.get_next_component() {
            if self.can_do_build_action(&next_task.action)
                && self.evaluate_condition(&next_task.prereq)
            {
                self.attempt_build_action(&next_task.action);
                println!("we started a build action!");
                //self.bom.mark_component_done();
            }
        }
    }

    fn can_do_build_action(&self, action: &BuildOrderAction) -> bool {
        match action {
            BuildOrderAction::Chrono(_) => self
                .units
                .my
                .townhalls
                .iter()
                .any(|n| n.energy().unwrap_or(0) >= CHRONOBOOST_COST),
            BuildOrderAction::Construct(building) => {
                let afford = self.can_afford(*building, true);
                let has_worker = !self.units.my.workers.is_empty();
                println!("afford: {afford}, has worker {has_worker}");
                afford && has_worker
            }
            BuildOrderAction::Research(upgrade, reseacher, _) => {
                self.units
                    .my
                    .structures
                    .of_type(*reseacher)
                    .idle()
                    .is_empty()
                    && self.can_afford_upgrade(*upgrade)
            }
            BuildOrderAction::Train(_, ability) => {
                let has_trainer = self
                    .units
                    .my
                    .structures
                    .iter()
                    .find(|s| s.has_ability(*ability))
                    .is_some();
                has_trainer
            }
        }
    }

    fn check_policies(&self) {
        let mut attempted_policies = 0;
        for policy in &self.bom.policies {
            if !policy.active {
                continue;
            }
            if !self.evaluate_condition(&policy.condition) {
                continue;
            }
            if !self.can_do_build_action(&policy.action) {
                continue;
            }
            self.attempt_build_action(&policy.action);
            //println!("Attempted Policy Action! {policy}");
            attempted_policies += 1;
        }

        if attempted_policies == 0 {
            //println!("No policies attempted");
        };
    }

    fn evaluate_condition(&self, condition: &BuildCondition) -> bool {
        match condition {
            BuildCondition::SupplyAtLeast(supply) => self.supply_used >= *supply,
            BuildCondition::SupplyBetween(low, high) => {
                self.supply_used >= *low && self.supply_used < *high
            }
            BuildCondition::LessThanCount(unit_type, desired_count) => {
                let unit_count = self.counter().count(*unit_type);
                unit_count < *desired_count
            }
            BuildCondition::SupplyLeft(remaining_supply) => self.supply_left < *remaining_supply,
            BuildCondition::StructureComplete(structure_type) => self
                .units
                .my
                .structures
                .iter()
                .any(|u| u.type_id() == *structure_type),
            BuildCondition::TechComplete(upgrade) => self.upgrade_progress(*upgrade) > 0.95,
        }
    }

    fn attempt_build_action(&self, action: &BuildOrderAction) {
        match action {
            BuildOrderAction::Construct(unit_type) => {
                self.build(unit_type);
            }
            BuildOrderAction::Train(unit_type, _) => self.train(*unit_type),
            BuildOrderAction::Chrono(ability) => self.chrono_boost(*ability),
            BuildOrderAction::Research(upgrade, researcher, ability) => {
                self.research(*researcher, *upgrade, *ability);
            }
        }
    }

    fn build(&self, structure_type: &UnitTypeId) {
        let position = self
            .base_managers
            .first()
            .unwrap()
            .siting_manager
            .get_free_building_site(3);
        if let Some(position) = position {
            let builder = self.units.my.workers.first().unwrap();
            builder.build(*structure_type, position.location, false);
            builder.sleep(5);
        }
    }

    fn train(&self, unit_type: UnitTypeId) {
        let trainer = self.units.my.townhalls.first().unwrap();
        trainer.train(unit_type, false);
    }

    fn chrono_boost(&self, ability: AbilityId) {
        let mut energetic_nexi = self
            .units
            .my
            .townhalls
            .iter()
            .filter(|unit| unit.energy().unwrap_or(0) >= CHRONOBOOST_COST);
        let target = self
            .units
            .my
            .structures
            .iter()
            .find(|s| s.is_using(ability));
        if let (Some(nexus), Some(target)) = (energetic_nexi.next(), target) {
            nexus.command(
                AbilityId::EffectChronoBoost,
                Target::Tag(target.tag()),
                false,
            );
        }
    }

    pub fn research(&self, researcher: UnitTypeId, upgrade: UpgradeId, ability: AbilityId) {
        let researchers = self.units.my.all.of_type(researcher).ready().idle();
        if !researchers.is_empty()
            && !self.has_upgrade(upgrade)
            && !self.is_ordered_upgrade(upgrade)
        {
            if let Some(candidate) = researchers.first() {
                if self.can_afford_upgrade(upgrade) {
                    candidate.use_ability(ability, true);
                }
            }
        }
    }
}
