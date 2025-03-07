use std::collections::HashSet;

use crate::base_manager::BaseManager;
use crate::build_order::BuildOrderManager;
use crate::siting::SitingDirector;
use crate::Tag;

use rust_sc2::prelude::*;
#[bot]
#[derive(Default)]
pub struct ReBiCycler {
    pub bom: BuildOrderManager,
    pub base_managers: Vec<BaseManager>,
    pub siting_director: SitingDirector,
    game_started: bool,
}
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        self.bom = BuildOrderManager::new();

        let map_center = self.game_info.map_center;

        self.siting_director
            .initialize_global_placement(self.expansions.clone().as_slice(), map_center);
        println!("Building templates placed: {:?}", self.siting_director);
        self.validate_building_locations();

        println!("Global siting complete: {:?}", self.siting_director);

        for worker in &self.units.my.workers.clone() {
            if self.reassign_worker_to_nearest_base(worker).is_err() {
                println!("No bases at game start?!");
            }
        }

        println!("Game start!");
        self.game_started = true;
        Ok(())
    }

    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        self.observe();

        //self.micro();
        if frame_no % 250 == 0 {
            println!(
                "Step step step {}, M:{}, G:{}, S:{}/{}",
                frame_no, self.minerals, self.vespene, self.supply_used, self.supply_cap
            );
            let unit_types: HashSet<UnitTypeId> =
                self.units.my.all.iter().map(Unit::type_id).collect();
            println!("Unit types: {unit_types:?}");

            self.step_build();
        };
        if frame_no >= 6000 && frame_no % 100 == 0 {
            if let Some(structure) = self.units.my.structures.first() {
                let _: () = self
                    .units
                    .my
                    .workers
                    .iter()
                    .idle()
                    .map(|w| w.attack(Target::Tag(structure.tag()), false))
                    .collect();

                let _: () = self
                    .units
                    .my
                    .workers
                    .iter()
                    .map(|w| {
                        if w.is_gathering() {
                            w.attack(Target::Tag(structure.tag()), false);
                        }
                    })
                    .collect();
            }
        }
        Ok(())
    }

    fn on_event(&mut self, event: Event) -> SC2Result<()> {
        match event {
            Event::ConstructionComplete(building_tag) => {
                let Some(building) = self.units.my.structures.get(building_tag) else {
                    println!("ConstructionComplete but unit not found! {building_tag}");
                    return Ok(());
                };

                println!(
                    "Building Finished! {:?}, {building_tag}",
                    building.type_id()
                );

                if building.type_id() == UnitTypeId::Nexus {
                    if let Err(e) = self.new_base_finished(building.position()) {
                        println!("Nexus build on non-expansion location! {e:?}");
                    }
                }
            }
            Event::UnitCreated(unit_tag) => {
                let Some(unit) = self.units.my.units.get(unit_tag).cloned() else {
                    println!("UnitCreated but unit not found! {unit_tag}");
                    return Ok(());
                };
                //println!("New Unit! {:?}, {}", unit.type_id(), unit_tag);
                if unit.type_id() == UnitTypeId::Probe && self.game_started {
                    if let Err(e) = self.reassign_worker_to_nearest_base(&unit) {
                        println!("Unable to assign new probe to a nexus? {e:?}");
                    }
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
                            if let Err(e) =
                                self.siting_director.find_and_destroy_building(&unit_tag)
                            {
                                println!(
                                    "Destroyed structure not logged in siting director! {e:?}"
                                );
                            };
                        }
                    }
                    None => println!("Unknown unit destroyed: {unit_tag:?}"),
                };
            }
            Event::ConstructionStarted(building_tag) => {
                let Some(building) = self.units.my.structures.get(building_tag).cloned() else {
                    println!("ConstructionStarted but building not found! {building_tag}");
                    return Ok(());
                };
                println!("New Building! {:?}, {building_tag}", building.type_id());
                let tag = Tag::from_unit(&building);
                if let Err(e) = self
                    .siting_director
                    .construction_begin(tag, building.position())
                {
                    println!("No slot for new building: {e:?}");
                }
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
}
