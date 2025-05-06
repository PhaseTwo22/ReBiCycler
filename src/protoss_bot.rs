use crate::army::ArmyController;
use crate::assignment_manager::{Commands, Identity};
use crate::build_order_definitions::nexus_first_two_base_charge;
use crate::build_tree::BuildOrderTree;
use crate::chatter::{ChatAction, ChatController};
use crate::construction::ConstructionManager;
use crate::errors::BuildError;
use crate::knowledge::Knowledge;
use crate::mining::MinerController;
use crate::readout::DisplayTerminal;
use crate::siting::SitingDirector;
use crate::Tag;

use rust_sc2::prelude::*;

const SURRENDER_DELAY_FRAMES: u32 = 200;

#[bot]
#[derive(Default)]
pub struct ReBiCycler {
    /// A tree data structure for our build orders
    pub build_order: BuildOrderTree,
    /// information about how we want to place buildings. TODO use a grid
    pub siting_director: SitingDirector,
    /// a place to store persistent knowledge about the game state
    pub knowledge: Knowledge,
    /// controls the army and assignments and stuff
    pub army_manager: ArmyController,
    /// manages workers and executes speed mining
    pub mining_manager: MinerController,
    /// Manages construction projects
    pub construction_manager: ConstructionManager,
    /// Does chat stuff.
    pub chat_controller: ChatController,
    /// a text terminal for what's going on inside the bot.
    /// gets saved after every game.
    pub display_terminal: DisplayTerminal,
    game_started: bool,
    pub bot_state: BotState,
}

/// These are the methods that the game will call that the bot must implement. They are the entry points into all the code we want to run.
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }

    /// called once at the start of the game, before the first frame
    fn on_start(&mut self) -> SC2Result<()> {
        self.build_order = nexus_first_two_base_charge();

        let map_center = self.game_info.map_center;

        self.siting_director.initialize_global_placement(
            self.expansions.clone().as_slice(),
            self.units.vespene_geysers.clone(),
            map_center,
        );

        self.game_started = true;
        self.do_chat(ChatAction::Greeting);
        Ok(())
    }

    /// called each frame of the game
    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        match self.bot_state {
            BotState::Surrendering(done_frame) => {
                if done_frame + SURRENDER_DELAY_FRAMES > self.game_step() {
                    println!("Surrendering. GG!");
                    if let Err(e) = self.on_end(GameResult::Defeat) {
                        println!("ending the game didn't go well: {e:?}");
                    }
                    let _ = self.leave();
                }
            }
            BotState::Nominal => (),
        }

        if frame_no == 0 {
            self.first_frame();
        }
        self.observe(frame_no);

        if frame_no % 50 == 0 {
            self.step_build();
            //self.map_worker_activity(frame_no);
        }

        self.update_managers();
        self.micro_managers();

        let updates = self
            .mining_manager
            .get_peon_updates(self.units.my.workers.clone());
        self.mining_manager.apply_peon_updates(updates);

        if frame_no % 250 == 0 {
            self.monitor(frame_no);
        };

        if frame_no % 1000 == 0 {
            self.map_siting(frame_no);
        }

        Ok(())
    }
    /// called each time a particular event happens
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
    /// called at the end of the game. maybe also call when surrendering
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
            build_order: BuildOrderTree::new(),
            game_started: false,
            ..Default::default()
        }
    }
    /// Not everything is ready before the game starts, do stuff that we need everything ready for
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
            if let Err(e) = self.mining_manager.add_resource(&mineral) {
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

    /// writes errors to our display rather than printing them, so we can stpre and ignore or whatever
    #[allow(clippy::needless_pass_by_value)]
    pub fn log_error(&mut self, message: String) {
        self.display_terminal
            .write_line_to_pane("Errors", &message, true);
    }
}

pub enum BotState {
    Nominal,
    Surrendering(u32),
}
impl Default for BotState {
    fn default() -> Self {
        Self::Nominal
    }
}

impl ReBiCycler {
    fn update_managers(&mut self) {
        let updates = self
            .mining_manager
            .get_peon_updates(self.units.my.workers.clone());
        self.mining_manager.apply_peon_updates(updates);
    }

    fn micro_managers(&self) {
        self.micro_single_manager(&self.mining_manager);
    }

    fn micro_single_manager(
        &self,
        manager: &impl Commands<(AbilityId, Target, bool), impl Identity<u64>, u64, Units>,
    ) {
        for (tag, (ability, target, queue)) in manager.issue_commands() {
            if let Some(unit) = self.units.my.workers.get(tag) {
                unit.command(ability, target, queue);
            }
        }
    }
}

impl Identity<u64> for Unit {
    fn id(&self) -> u64 {
        self.tag()
    }
}
