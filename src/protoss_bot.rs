use crate::build_order_manager::BuildOrder;
use crate::build_orders::four_base_charge;
use crate::knowledge::Knowledge;
use crate::micro::MinerManager;
use crate::readout::DisplayTerminal;
use crate::siting::SitingDirector;
use crate::{is_assimilator, Tag};

use rust_sc2::prelude::*;
#[bot]
#[derive(Default)]
pub struct ReBiCycler {
    pub build_order: BuildOrder,
    pub siting_director: SitingDirector,
    pub knowledge: Knowledge,
    pub mining_manager: MinerManager,
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

        self.siting_director.initialize_global_placement(
            self.expansions.clone().as_slice(),
            self.units.vespene_geysers.clone(),
            map_center,
        );
        println!("Building templates placed: {:?}", self.siting_director);
        self.update_building_obstructions();

        println!("Global siting complete: {:?}", self.siting_director);

        println!("Game start!");
        self.game_started = true;
        Ok(())
    }

    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        if frame_no == 0 {
            self.first_frame();
        }
        self.observe(frame_no);
        self.broadcast_alerts();

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
                let building = building.clone();
                if let Err(e) = self.siting_director.finish_construction(&building) {
                    println!("Error finishing building: {e:?}");
                };

                if building.type_id() == UnitTypeId::Nexus {
                    if let Err(e) = self.new_base_finished(&building.clone()) {
                        println!("BaseManager failed to initialize: {e:?}");
                    }
                } else if building.type_id() == UnitTypeId::Pylon {
                    if let Err(e) =
                        self.update_building_power(UnitTypeId::Pylon, building.position(), true)
                    {
                        println!(
                            "Error in powering buildings when Pylon {building_tag:?} completed: {e:?}"
                        );
                    }
                } else if is_assimilator(building.type_id()) {
                    let bc = building.clone();
                    if let Err(e) = self.mining_manager.add_resource(bc) {
                        println!("I expected this to be harvestable: {e:?}");
                    };
                }
            }
            Event::UnitCreated(unit_tag) => {
                let Some(unit) = self.units.my.units.get(unit_tag).cloned() else {
                    println!("UnitCreated but unit not found! {unit_tag}");
                    return Ok(());
                };
                //println!("New Unit! {:?}, {}", unit.type_id(), unit_tag);
                if unit.type_id() == UnitTypeId::Probe && self.game_started {
                    if let Err(e) = self.back_to_work(&unit) {
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
                        unit_type: unit_details.type_id,
                    };
                    if crate::is_assimilator(unit_details.type_id) {
                        if self.siting_director.lose_assimilator(unit_tag).is_err() {
                            println!("We couldn't find the assimilator to destroy");
                        }
                    } else if crate::is_protoss_building(unit_details.type_id) {
                        if let Err(e) = self.siting_director.find_and_destroy_building(&unit_tag) {
                            println!("Destroyed structure not logged in siting director! {e:?}");
                        };

                        if unit_details.type_id == UnitTypeId::Pylon
                            || unit_details.type_id == UnitTypeId::WarpPrismPhasing
                        {
                            if let Err(e) = self.update_building_power(
                                unit_details.type_id,
                                unit_details.last_position,
                                false,
                            ) {
                                println!("Error depowering pylon: {e:?}");
                            }
                        }
                    } else if crate::is_minerals(unit_details.type_id) {
                        let unemployed: Vec<Unit> = self
                            .mining_manager
                            .remove_resource(unit_tag.tag)
                            .into_iter()
                            .filter_map(|u| self.units.my.workers.get(u))
                            .cloned()
                            .collect();

                        for unit in unemployed {
                            self.back_to_work(&unit.clone());
                        }
                    } else if unit_tag.unit_type == UnitTypeId::Probe
                        && self.mining_manager.remove_miner(unit_tag.tag)
                    {
                        println!("Dead worker was mining");
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
                    if let Err(problem) = self.siting_director.add_assimilator(&building) {
                        println!(
                            "Nowhere could place the assimilator we just started. {problem:?}"
                        );
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

impl ReBiCycler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            /* initializing fields */
            build_order: BuildOrder::empty(),
            game_started: false,
            ..Default::default()
        }
    }

    fn first_frame(&mut self) {
        let nearby_minerals: Vec<Unit> = self
            .units
            .resources
            .iter()
            .closer(10.0, self.start_center)
            .filter(|u| u.is_mineral())
            .cloned()
            .collect();
        for mineral in nearby_minerals {
            if let Err(e) = self.mining_manager.add_resource(mineral) {
                println!("Error adding initial minerals: {e:?}");
            };
        }

        for worker in &self.units.my.workers.clone() {
            if self.back_to_work(worker).is_err() {
                println!("No bases at game start?!");
            }
        }
    }

    fn broadcast_alerts(&self) {
        let alerts = &self.state.observation.alerts;
        if !alerts.is_empty() {
            println!("ALERTS: {alerts:?}");
        }

        let abilities: Vec<AbilityId> = self
            .state
            .observation
            .abilities
            .iter()
            .map(|a| a.id)
            .collect();
        if !abilities.is_empty() {
            println!("Available Abilities: {abilities:?}");
            panic!("The state.observation.abilities field was filled in for once!")
        }
    }
}
