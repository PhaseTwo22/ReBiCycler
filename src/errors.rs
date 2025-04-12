use std::fmt::Debug;

use rust_sc2::{ids::AbilityId, prelude::Point2};

use crate::{
    siting::{BuildingStatus, BuildingTransition},
    Tag,
};

pub struct UnitEmploymentError(pub String);
impl Debug for UnitEmploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in employment: {}", self.0)
    }
}

pub struct InvalidUnitError(pub String);
impl Debug for InvalidUnitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad unit: {}", self.0)
    }
}

#[derive(Debug)]
pub enum BuildError {
    CantPlace(Point2, rust_sc2::ids::UnitTypeId),
    CantTransitionBuildingLocation(BuildingTransitionError),
    NoBuildingLocationHere(Point2),
    NoBuildingLocationForFinishedBuilding,
    NoPlacementLocations,
    CantAfford,
    InvalidUnit(String),
    NoTrainer,
    AllBusy(AbilityId),
    AllChronoed(AbilityId),
    AlreadyResearching,
    NoBuildItemsLeft,
    WarpGateNotResearched,
    NoPower(Point2),
}
#[derive(Debug)]
pub enum BuildingTransitionError {
    InvalidTransition {
        from: BuildingStatus,
        change: BuildingTransition,
    },
    InvalidUnit,
}

#[derive(Debug)]
pub enum MicroError {
    UnitNotRegistered(Tag),
}

pub type UnhandledError = String;
