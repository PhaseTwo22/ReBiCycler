use std::fmt::{Debug, Display};

use rust_sc2::{
    ids::{AbilityId, UnitTypeId, UpgradeId},
    prelude::Point2,
};

use crate::{
    siting::{BuildingStatus, BuildingTransition},
    Assigns, Tag,
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
    NoConstructionSiteHere(Point2),
    NoConstructionSiteForFinishedBuilding(UnitTypeId),
    NoPlacementLocations,
    CantAfford,
    InvalidUnit(String),
    NoTrainer,
    NoResearcher(UpgradeId),
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

pub struct AssignmentError {
    assignee: Tag,
    manager: String,
    reason: AssignmentIssue,
}

impl AssignmentError {
    pub fn new(assignee: Tag, manager: String, reason: AssignmentIssue) -> Self {
        Self {
            assignee,
            manager: manager.to_string(),
            reason,
        }
    }
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Can't assign {} to {}: {:?}",
            self.assignee, self.manager, self.reason
        )
    }
}

#[derive(Debug)]
pub enum AssignmentIssue {
    InvalidUnit,
    UnitAlreadyAssigned,
    UnitNotAssigned,
    DifferentUnitAssignedInRole,
}
