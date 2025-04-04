use std::f32::consts::TAU;

use rust_sc2::{game_state::PsionicMatrix, prelude::*};

use crate::{
    build_orders::{BuildCondition, BuildOrderAction, BuildOrderComponent},
    errors::BuildError,
    protoss_bot::ReBiCycler,
};

/// This module serves to manage our build orders.
/// We want to use kiss principle here, but still have a flexible system.
///
/// A build order is made of components, each with a condition and an action.
/// It also contains policies, which are a set of actions that are executed until a condition is met.
/// Policies can be active and inactive.

#[derive(Default)]
pub struct BuildOrder(pub Vec<BuildOrderComponent>);

impl BuildOrder {
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &BuildOrderComponent> {
        self.0.iter()
    }
}

impl ReBiCycler {
    pub fn step_build(&mut self) {
        self.progress_build();
    }

    fn progress_build(&mut self) {
        let started_tasks = self
            .build_order
            .iter()
            .filter(|boc| self.evaluate_conditions(&boc.start_conditions));
        let started_and_not_finished =
            started_tasks.filter(|boc| !self.evaluate_conditions(&boc.end_conditions));

        let valid_and_doable: Vec<BuildOrderAction> = started_and_not_finished
            .filter_map(|boc| {
                if self.can_do_build_action(boc.action) {
                    Some(boc.action)
                } else {
                    None
                }
            })
            .collect();

        for action in valid_and_doable {
            self.attempt_build_action(action);
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
            BuildOrderAction::Construct(building) => {
                let afford = self.can_afford(building, true);
                let has_worker = !self.units.my.workers.is_empty();
                afford && has_worker
            }
            BuildOrderAction::Research(upgrade, reseacher, _) => {
                self.units
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
        }
    }

    fn evaluate_conditions(&self, conditions: &[BuildCondition]) -> bool {
        conditions.iter().all(|condition| match condition {
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
        })
    }

    fn attempt_build_action(&mut self, action: BuildOrderAction) {
        //println!("Attempting a policy! {action:?}");
        let result = match action {
            BuildOrderAction::Expand => {
                self.validate_building_locations();
                self.build(UnitTypeId::Nexus)
            }
            BuildOrderAction::Construct(UnitTypeId::Assimilator) => self.build_gas(),
            BuildOrderAction::Construct(unit_type) => {
                self.validate_building_locations();
                self.build(unit_type)
            }
            BuildOrderAction::Train(unit_type, ablilty) => self.train(unit_type, ability),
            BuildOrderAction::Chrono(ability) => self.chrono_boost(ability),
            BuildOrderAction::Research(upgrade, researcher, ability) => {
                self.research(researcher, upgrade, ability)
            }
        };

        if let Err(err) = result {
            match err {
                BuildError::CantPlace(location, _type_id) => {
                    if let Err(err) = self.siting_director.mark_position_blocked(location) {
                        println!("Can't block non-templated building location: {err:?}");
                    };
                }
                _ => println!("Build order blocked: {action:?} > {err:?}"),
            }
        } else {
            println!("BuildOrderAction OK: {action:?}");
        }
    }

    fn train(&self, unit_type: UnitTypeId, ability: AbilityId) -> Result<(), BuildError> {
        let trainer = self.units
                .my
                .structures
                .idle()
                .iter()
                .filter(|s| s.has_ability(ability))
                .next()
                .ok_or(BuildError::NoTrainer)?;

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

    fn warp_spot_spiral_search(
        &self,
        matrix: &PsionicMatrix,
        warpgate: &Unit,
        unit_type: UnitTypeId,
        unit_width: f32,
    ) -> Result<(), BuildError> {
        let possible_rings = (matrix.radius / unit_width).floor();
        for ring_number in 1..possible_rings as i32 {
            let circumference = TAU * matrix.radius * (ring_number as f32 / possible_rings);
            let possible_angles = (circumference / unit_width).floor();
            for angle_step in 0..possible_angles as i32 {
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
            .ok_or(BuildError::NoTrainer)?;

        nexus.command(
            AbilityId::EffectChronoBoostEnergyCost,
            Target::Tag(target.tag()),
            false,
        );
        println!(
            "Chrono! {:?} on {:?}",
            crate::Tag::from_unit(nexus),
            crate::Tag::from_unit(target),
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
        researcher: UnitTypeId,
        upgrade: UpgradeId,
        ability: AbilityId,
    ) -> Result<(), BuildError> {
        if self.has_upgrade(upgrade) && self.is_ordered_upgrade(upgrade) {
            Err(BuildError::AlreadyResearching)
        } else {
            let researcher = self
                .units
                .my
                .all
                .iter()
                .of_type(researcher)
                .ready()
                .idle()
                .next()
                .ok_or(BuildError::NoTrainer)?;

            if self.can_afford_upgrade(upgrade) {
                researcher.use_ability(ability, true);
                Ok(())
            } else {
                Err(BuildError::CantAfford)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    const fn test_build_order_manager() {}
}
