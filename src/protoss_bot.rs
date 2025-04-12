use crate::base_manager::BaseManager;
use crate::build_order_manager::BuildOrder;
use crate::build_orders::four_base_charge;
use crate::errors::BuildError;
use crate::knowledge::Knowledge;
use crate::readout::DisplayTerminal;
use crate::siting::SitingDirector;
use crate::Tag;

use rust_sc2::prelude::*;
#[bot]
#[derive(Default)]
pub struct ReBiCycler {
    pub build_order: BuildOrder,
    pub base_managers: Vec<BaseManager>,
    pub siting_director: SitingDirector,
    pub knowledge: Knowledge,
    pub display_terminal: DisplayTerminal,
    game_started: bool,
}
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        self.build_order = four_base_charge();

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
        self.observe(frame_no);

        if frame_no % 50 == 0 {
            self.step_build();
        }

        //self.micro();
        if frame_no % 250 == 0 {
            self.monitor(frame_no);
        };

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
                        println!("BaseManager failed to initialize: {e:?}");
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
            Event::UnitDestroyed(unit_tag, _alliance) => {
                let knowledge = self.knowledge.unit_destroyed(unit_tag);

                if let Ok(unit_details) = knowledge {
                    println!(
                        "Perished! {:?} {:?}",
                        unit_details.alliance, unit_details.type_id
                    );
                    let unit_tag = Tag {
                        tag: unit_tag,
                        type_id: unit_details.type_id,
                    };
                    if crate::is_assimilator(unit_details.type_id) {
                        let none_found = self
                            .base_managers
                            .iter_mut()
                            .map(|bm| bm.unassign_unit(&unit_tag))
                            .all(|x| x.is_err());
                        if none_found {
                            println!("We couldn't find the assimilator to destroy");
                        }
                    } else if crate::is_protoss_building(unit_details.type_id) {
                        if let Err(e) = self.siting_director.find_and_destroy_building(&unit_tag) {
                            println!("Destroyed structure not logged in siting director! {e:?}");
                        };
                    }
                }
            }
            Event::ConstructionStarted(building_tag) => {
                let Some(building) = self.units.my.structures.get(building_tag).cloned() else {
                    println!("ConstructionStarted but building not found! {building_tag}");
                    return Ok(());
                };
                println!("New Building! {:?}, {building_tag}", building.type_id());
                let tag = Tag::from_unit(&building);

                if (building.type_id() == UnitTypeId::Assimilator)
                    | (building.type_id() == UnitTypeId::AssimilatorRich)
                {
                    let add_attempts: Vec<Result<(), BuildError>> = self
                        .base_managers
                        .iter_mut()
                        .map(|bm| bm.add_building(&building))
                        .collect();

                    if !add_attempts.iter().any(Result::is_ok) {
                        println!("Nowhere could place the assimilator we just started.");
                    }
                } else if let Err(e) = self
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
