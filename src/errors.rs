use std::fmt::Debug;

use rust_sc2::prelude::Point2;

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
    NoBuildingLocationHere(Point2),
    NoPlacementLocations,
    CantAfford,
    InvalidUnit(String),
    NoTrainer,
    AlreadyResearching,
    NoBuildItemsLeft,
    WarpGateNotResearched,
}
