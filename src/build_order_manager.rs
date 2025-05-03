use std::f32::consts::TAU;

use itertools::Either;
use rust_sc2::{game_state::PsionicMatrix, prelude::*};

use crate::{
    build_orders::{BuildCondition, BuildOrderAction},
    errors::{BuildError, BuildingTransitionError},
    protoss_bot::{BotState, ReBiCycler},
};

impl ReBiCycler {
    pub fn step_build(&mut self) {
        let doable_components = self.update_build();
        for action in doable_components
            .iter()
            .filter_map(super::build_tree::BuildComponent::action)
        {
            if self.can_do_build_action(action) {
                self.attempt_build_action(action);
            }
        }
    }

    fn can_do_build_action(&self, action: BuildOrderAction) -> bool {
        match action {
            BuildOrderAction::Expand => self.can_afford(UnitTypeId::Nexus, false),
            BuildOrderAction::Chrono(_) => self
                .units
                .my
                .townhalls
                .iter()
                .any(|n| n.has_ability(AbilityId::EffectChronoBoostEnergyCost)),
            BuildOrderAction::ChronoWhatever(trainer) => {
                self.units
                    .my
                    .townhalls
                    .iter()
                    .any(|n| n.has_ability(AbilityId::EffectChronoBoostEnergyCost))
                    && self
                        .units
                        .my
                        .structures
                        .iter()
                        .of_type(trainer)
                        .any(rust_sc2::prelude::Unit::is_active)
            }
            BuildOrderAction::Construct(building) => {
                let afford = self.can_afford(building, true);
                let has_worker = !self.units.my.workers.is_empty();
                afford && has_worker
            }
            BuildOrderAction::Research(upgrade, _, reseacher) => {
                !self
                    .units
                    .my
                    .structures
                    .of_type(reseacher)
                    .idle()
                    .is_empty()
                    && self.can_afford_upgrade(upgrade)
            }
            BuildOrderAction::Train(_, ability) => {
                let has_trainer = self
                    .units
                    .my
                    .structures
                    .iter()
                    .any(|s| s.has_ability(ability));
                has_trainer
            }
            BuildOrderAction::Surrender => true,
            BuildOrderAction::Chat(_) => true,
        }
    }

    pub fn evaluate_condition(&self, condition: &BuildCondition) -> bool {
        match condition {
            BuildCondition::DontHaveAnyDone(unit) => self.counter().count(*unit) == 0,
            BuildCondition::DontHaveAnyStarted(unit) => self.counter().ordered().count(*unit) == 0,
            BuildCondition::SupplyAtLeast(supply) => self.supply_used >= *supply,
            BuildCondition::SupplyBetween(low, high) => {
                self.supply_used >= *low && self.supply_used < *high
            }
            BuildCondition::LessThanCount(unit_type, desired_count) => {
                let unit_count = self.counter().all().count(*unit_type);
                unit_count < *desired_count
            }
            BuildCondition::SupplyLeftBelow(remaining_supply) => {
                self.supply_left < *remaining_supply
            }
            BuildCondition::StructureComplete(structure_type) => {
                self.units
                    .my
                    .structures
                    .of_type(*structure_type)
                    .iter()
                    .ready()
                    .count()
                    > 0
            }
            BuildCondition::TechComplete(upgrade) => self.upgrade_progress(*upgrade) > 0.95,
            BuildCondition::AtLeastCount(unit_type, desired_count) => {
                self.counter().all().count(*unit_type) >= *desired_count
            }
            BuildCondition::Never => false,
            BuildCondition::Always => true,
            BuildCondition::TotalAndOrderedAtLeast(thing, count) => {
                self.counter().all().count(*thing) >= *count
            }
        }
    }

    fn attempt_build_action(&mut self, action: BuildOrderAction) {
        let result = match action {
            BuildOrderAction::Expand => {
                let issues = self.update_building_obstructions();
                let _: () = issues
                    .into_iter()
                    .map(|e| self.unhandle_build(e, action))
                    .collect();
                self.build(UnitTypeId::Nexus)
            }
            BuildOrderAction::Construct(UnitTypeId::Assimilator) => self.build_gas(),
            BuildOrderAction::Construct(unit_type) => {
                let issues = self.update_building_obstructions();
                let _: () = issues
                    .into_iter()
                    .map(|e| self.unhandle_build(e, action))
                    .collect();
                self.build(unit_type)
            }
            BuildOrderAction::Train(unit_type, ability) => self.train(unit_type, ability),
            BuildOrderAction::Chrono(ability) => self.chrono_boost(ability),
            BuildOrderAction::ChronoWhatever(trainer) => self.chrono_whatever(trainer),
            BuildOrderAction::Research(upgrade, ability, researcher) => {
                self.display_terminal.write_line_to_footer(&format!(
                    "Attempting build action to {ability:?} for {upgrade:?}"
                ));
                self.research(upgrade, ability, researcher)
            }
            BuildOrderAction::Surrender => {
                self.bot_state = BotState::Surrendering(self.game_step());
                println!("Surrendering. GG!");
                if let Err(e) = self.on_end(GameResult::Defeat) {
                    println!("ending the game didn't go well: {e:?}");
                }
                let _ = self.leave();
                Ok(())
            }
            BuildOrderAction::Chat(chat) => {
                self.do_chat(chat);
                Ok(())
            }
        };

        if let Err(err) = result {
            match err {
                BuildError::CantPlace(location, _type_id) => {
                    if let Err(err) = self.siting_director.mark_position_blocked(
                        location,
                        crate::siting::BuildingTransition::Obstruct,
                    ) {
                        self.unhandle_build(err, action);
                    } else {
                        // bad location marked blocked, no problem.
                    }
                }
                BuildError::AllBusy(_) | BuildError::AllChronoed(_) => (),
                _ => self.unhandle_build(Either::Left(err), action),
            }
        }
    }

    pub fn unhandle_build(
        &mut self,
        err: Either<BuildError, BuildingTransitionError>,
        action: BuildOrderAction,
    ) {
        let error_part = err.map_either(|x| format!("{x:?}"), |y| format!("{y:?}"));
        let message = format!("Build error not yet handled: {action:?} from {error_part:?}");
        self.display_terminal
            .write_line_to_pane("Errors", &message, true);
    }

    fn train(&self, unit_type: UnitTypeId, ability: AbilityId) -> Result<(), BuildError> {
        let mut trainers = self
            .units
            .my
            .structures
            .iter()
            .filter(|s| s.has_ability(ability))
            .peekable();
        trainers.peek().ok_or(BuildError::NoTrainer)?;
        let trainer = trainers
            .find(|u| u.is_idle())
            .ok_or(BuildError::AllBusy(ability))?;

        if trainer.type_id() == UnitTypeId::WarpGate {
            self.warp_in(unit_type, trainer)
        } else {
            trainer.train(unit_type, false);
            Ok(())
        }
    }

    fn warp_in(&self, unit_type: UnitTypeId, warpgate: &Unit) -> Result<(), BuildError> {
        let unit_width = 2.0;
        let booster_structures = self.units.my.all.of_types(&vec![
            UnitTypeId::Pylon,
            UnitTypeId::Nexus,
            UnitTypeId::WarpPrismPhasing,
        ]);

        let is_fast = |matrix: &&PsionicMatrix| {
            !booster_structures
                .closer(matrix.radius, matrix.pos)
                .is_empty()
        };

        let fast_warpins = self
            .state
            .observation
            .raw
            .psionic_matrix
            .iter()
            .filter(is_fast);

        for matrix in fast_warpins {
            if self
                .warp_spot_spiral_search(matrix, warpgate, unit_type, unit_width)
                .is_ok()
            {
                return Ok(());
            }
        }
        Err(BuildError::NoPlacementLocations)
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
    fn warp_spot_spiral_search(
        &self,
        matrix: &PsionicMatrix,
        warpgate: &Unit,
        unit_type: UnitTypeId,
        unit_width: f32,
    ) -> Result<(), BuildError> {
        let possible_rings = (matrix.radius / unit_width).floor();
        for ring_number in 1..possible_rings as u64 {
            let circumference = TAU * matrix.radius * (ring_number as f32 / possible_rings);
            let possible_angles = (circumference / unit_width).floor();
            for angle_step in 0..possible_angles as usize {
                let angle = TAU * angle_step as f32 / possible_angles;
                let offset = Point2::new(ring_number as f32 * unit_width, 0.0).rotate(angle);

                let spot = matrix.pos + offset;
                if self.is_pathable(spot) && self.is_placeable(spot) {
                    warpgate.warp_in(unit_type, spot);
                    return Ok(());
                }
            }
        }
        Err(BuildError::NoPlacementLocations)
    }
    fn chrono_whatever(&self, trainer: UnitTypeId) -> Result<(), BuildError> {
        let nexus = self
            .units
            .my
            .structures
            .iter()
            .find(|unit| unit.has_ability(AbilityId::EffectChronoBoostEnergyCost))
            .ok_or(BuildError::CantAfford)?;
        let target = self
            .units
            .my
            .structures
            .iter()
            .of_type(trainer)
            .find(|s| s.is_active() && !s.has_buff(BuffId::ChronoBoostEnergyCost))
            .ok_or(BuildError::AllChronoed(AbilityId::YamatoYamatoGun))?;

        nexus.command(
            AbilityId::EffectChronoBoostEnergyCost,
            Target::Tag(target.tag()),
            false,
        );
        Ok(())
    }
    fn chrono_boost(&self, ability: AbilityId) -> Result<(), BuildError> {
        let nexus = self
            .units
            .my
            .structures
            .iter()
            .find(|unit| unit.has_ability(AbilityId::EffectChronoBoostEnergyCost))
            .ok_or(BuildError::CantAfford)?;
        let target = self
            .units
            .my
            .structures
            .iter()
            .find(|s| s.is_using(ability) && !s.has_buff(BuffId::ChronoBoostEnergyCost))
            .ok_or(BuildError::AllChronoed(ability))?;

        nexus.command(
            AbilityId::EffectChronoBoostEnergyCost,
            Target::Tag(target.tag()),
            false,
        );
        Ok(())
    }

    /// Finds a structure to do the research, and then does so.
    ///
    /// # Errors
    /// `BuildError::AlreadyResearching` if its already in progress or done
    /// `BuildError::CantAfford` if we can't afford the upgrade
    /// `BuildError::NoTrainer` if the required structure is destroyed. Not sure about depowered.
    pub fn research(
        &self,
        upgrade: UpgradeId,
        ability: AbilityId,
        researcher: UnitTypeId,
    ) -> Result<(), BuildError> {
        println!("trying to start research {ability:?}");
        if self.has_upgrade(upgrade) && self.is_ordered_upgrade(upgrade) {
            Err(BuildError::AlreadyResearching)
        } else if self.can_afford_upgrade(upgrade) {
            let researcher = self
                .units
                .my
                .all
                .iter()
                .of_type(researcher)
                .idle()
                .find(|u| u.has_ability(ability))
                .ok_or(BuildError::NoTrainer)?;

            researcher.use_ability(ability, true);
            Ok(())
        } else {
            Err(BuildError::CantAfford)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    const fn test_build_order_manager() {}
}
