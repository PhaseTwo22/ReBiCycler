use crate::build_order_manager::BuildOrder;
use crate::build_orders::four_base_charge;
use crate::errors::BuildError;
use crate::knowledge::Knowledge;
use crate::micro::MinerManager;
use crate::readout::DisplayTerminal;
use crate::siting::SitingDirector;
use crate::Tag;

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

        self.game_started = true;
        Ok(())
    }

    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        if frame_no == 0 {
            self.first_frame();
        }
        self.observe(frame_no);

        if frame_no % 50 == 0 {
            self.step_build();
        }

        //self.micro();
        if frame_no % 250 == 0 {
            self.monitor(frame_no);
        };

        if frame_no % 1000 == 0 {
            self.map_siting(frame_no);
            self.map_worker_activity(frame_no);
        }

        Ok(())
    }

    fn on_event(&mut self, event: Event) -> SC2Result<()> {
        match event {
            Event::ConstructionComplete(building_tag) => self.complete_construction(building_tag),
            Event::UnitCreated(unit_tag) => {
                self.create_unit(unit_tag);
            }
            Event::UnitDestroyed(unit_tag, alliance) => {
                self.unit_destroyed(unit_tag, alliance);
            }
            Event::ConstructionStarted(building_tag) => {
                self.start_construction(building_tag);
            }
            Event::RandomRaceDetected(race) => {
                self.knowledge.confirm_race(race);
            }
        }
        Ok(())
    }

    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        let _ = self.display_terminal.save_history("replays/history.txt");
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
                self.log_error(format!("Error adding initial minerals: {e:?}"));
            };
        }

        for worker in &self.units.my.workers.clone() {
            self.back_to_work(worker.tag());
        }

        let initial_slotting = self
            .units
            .my
            .townhalls
            .first()
            .ok_or_else(|| BuildError::InvalidUnit("No nexus at game start?!".to_string()))
            .cloned();
        let inserted = match initial_slotting {
            Ok(nexus) => self.siting_director.add_initial_nexus(&nexus),
            Err(e) => Err(e),
        };

        if let Err(e) = inserted {
            self.log_error(format!("Can't place nexus in initial buildingslot: {e:?}"));
        }
    }

    fn unit_destroyed(&mut self, tag: u64, _alliance: Option<Alliance>) {
        let knowledge = self.knowledge.unit_destroyed(tag);

        if let Ok(unit_details) = knowledge {
            let unit_tag = Tag {
                tag,
                unit_type: unit_details.type_id,
            };
            if crate::is_assimilator(unit_details.type_id) {
                if self.siting_director.lose_assimilator(unit_tag).is_err() {
                    self.log_error("We couldn't find the assimilator to destroy".to_string());
                }
                let unemployed = self.mining_manager.remove_resource(unit_tag.tag, false);
                for worker_tag in &unemployed {
                    self.back_to_work(*worker_tag);
                }
            } else if crate::is_protoss_building(&unit_details.type_id) {
                if let Err(e) = self.siting_director.find_and_destroy_building(&unit_tag) {
                    println!("Destroyed structure not logged in siting director! {e:?}");
                };

                if unit_details.type_id == UnitTypeId::Pylon
                    || unit_details.type_id == UnitTypeId::WarpPrismPhasing
                {
                    self.update_building_power(
                        unit_details.type_id,
                        unit_details.last_position,
                        false,
                    );
                }
                if unit_details.type_id == UnitTypeId::Nexus {
                    let unemployed = self.mining_manager.remove_townhall(unit_tag.tag);
                    for unit in unemployed {
                        self.back_to_work(unit);
                    }
                }
            } else if crate::is_minerals(unit_details.type_id) {
                let unemployed = self.mining_manager.remove_resource(unit_tag.tag, true);

                for unit in unemployed {
                    self.back_to_work(unit);
                }
            } else if unit_tag.unit_type == UnitTypeId::Probe
                && self.mining_manager.remove_miner(unit_tag.tag)
            {
            }
        }
    }

    fn create_unit(&mut self, unit_tag: u64) {
        if let Some(unit) = self.units.my.units.get(unit_tag).cloned() {
            if unit.type_id() == UnitTypeId::Probe && self.game_started {
                self.back_to_work(unit_tag);
            }
        } else {
            self.log_error(format!("UnitCreated but unit not found! {unit_tag}"));
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn log_error(&mut self, message: String) {
        self.display_terminal
            .write_line_to_pane("Errors", &message, true);
    }
}
