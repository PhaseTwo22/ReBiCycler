use rust_sc2::prelude::*;
use std::fmt::Debug;

mod base_manager;
mod build_order_manager;
mod build_orders;
mod errors;
mod knowledge;
mod monitor;
pub mod protoss_bot;
mod readout;
mod siting;
#[must_use]
pub fn get_options<'a>() -> LaunchOptions<'a> {
    LaunchOptions::<'a> {
        realtime: false,
        save_replay_as: Some("/home/andrew/Documents/rebicycler/replays/test.SC2Replay"),
        ..Default::default()
    }
}

#[must_use]
pub fn distance_squared(a: &Point2, b: &Point2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx.mul_add(dx, dy * dy)
}

#[must_use]
pub fn closest_index<T: Iterator<Item = Point2>>(target: Point2, population: T) -> Option<usize> {
    population
        .map(|pop| distance_squared(&target, &pop))
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

pub fn closest_point<T: Iterator<Item = Point2>>(target: Point2, population: T) -> Option<Point2> {
    population
        .map(|pop| (pop, distance_squared(&target, &pop)))
        .min_by(|(_pointa, dista), (_pointb, distb)| dista.total_cmp(distb))
        .map(|(point, _dist)| point)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Tag {
    tag: u64,
    type_id: UnitTypeId,
}
impl Tag {
    #[must_use]
    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            tag: unit.tag(),
            type_id: unit.type_id(),
        }
    }
    #[must_use]
    pub const fn default() -> Self {
        Self {
            tag: 0,
            type_id: UnitTypeId::NotAUnit,
        }
    }
}
#[must_use]
pub const fn is_protoss_building(unit: UnitTypeId) -> bool {
    matches!(
        unit,
        UnitTypeId::Nexus
            | UnitTypeId::Assimilator
            | UnitTypeId::AssimilatorRich
            | UnitTypeId::Pylon
            | UnitTypeId::Gateway
            | UnitTypeId::WarpGate
            | UnitTypeId::Forge
            | UnitTypeId::FleetBeacon
            | UnitTypeId::TwilightCouncil
            | UnitTypeId::PhotonCannon
            | UnitTypeId::Stargate
            | UnitTypeId::TemplarArchive
            | UnitTypeId::DarkShrine
            | UnitTypeId::RoboticsBay
            | UnitTypeId::RoboticsFacility
            | UnitTypeId::CyberneticsCore
            | UnitTypeId::ShieldBattery
    )
}
#[must_use]
pub const fn is_assimilator(unit: UnitTypeId) -> bool {
    matches!(unit, UnitTypeId::Assimilator | UnitTypeId::AssimilatorRich)
}

#[must_use]
pub const fn is_protoss_production(unit: UnitTypeId) -> bool {
    matches!(
        unit,
        UnitTypeId::Nexus
            | UnitTypeId::Gateway
            | UnitTypeId::WarpGate
            | UnitTypeId::Stargate
            | UnitTypeId::RoboticsBay
    )
}

#[must_use]
pub const fn is_protoss_tech(unit: UnitTypeId) -> bool {
    matches!(
        unit,
        UnitTypeId::Forge
            | UnitTypeId::FleetBeacon
            | UnitTypeId::TwilightCouncil
            | UnitTypeId::TemplarArchive
            | UnitTypeId::DarkShrine
            | UnitTypeId::RoboticsBay
            | UnitTypeId::CyberneticsCore
    )
}

fn building_names(unit: &UnitTypeId) -> String {
    match unit {
        UnitTypeId::Nexus => "Nexus",
        UnitTypeId::Assimilator => "Assimilator",
        UnitTypeId::AssimilatorRich => "AssimilatorRich",
        UnitTypeId::Pylon => "Pylon",
        UnitTypeId::Gateway => "Gateway",
        UnitTypeId::WarpGate => "WarpGate",
        UnitTypeId::Forge => "Forge",
        UnitTypeId::FleetBeacon => "FleetBeacon",
        UnitTypeId::TwilightCouncil => "TwilightCouncil",
        UnitTypeId::PhotonCannon => "PhotonCannon",
        UnitTypeId::Stargate => "Stargate",
        UnitTypeId::TemplarArchive => "TemplarArchive",
        UnitTypeId::DarkShrine => "DarkShrine",
        UnitTypeId::RoboticsBay => "RoboticsBay",
        UnitTypeId::RoboticsFacility => "RoboticsFacility",
        UnitTypeId::CyberneticsCore => "CyberneticsCore",
        UnitTypeId::ShieldBattery => "ShieldBattery",
        _ => "not implemented",
    }
    .to_string()
}

const fn ability_produces(ability: &AbilityId) -> UnitTypeId {
    match ability {
        AbilityId::TrainAdept => UnitTypeId::Adept,
        AbilityId::GatewayTrainZealot => UnitTypeId::Zealot,
        AbilityId::GatewayTrainSentry => UnitTypeId::Sentry,
        AbilityId::GatewayTrainStalker => UnitTypeId::Stalker,
        AbilityId::GatewayTrainDarkTemplar => UnitTypeId::DarkTemplar,
        AbilityId::GatewayTrainHighTemplar => UnitTypeId::HighTemplar,
        AbilityId::TrainDisruptor => UnitTypeId::Disruptor,
        AbilityId::RoboticsFacilityTrainColossus => UnitTypeId::Colossus,
        AbilityId::RoboticsFacilityTrainWarpPrism => UnitTypeId::WarpPrism,
        AbilityId::RoboticsFacilityTrainObserver => UnitTypeId::Observer,
        AbilityId::RoboticsFacilityTrainImmortal => UnitTypeId::Immortal,
        AbilityId::StargateTrainTempest => UnitTypeId::Tempest,
        AbilityId::StargateTrainOracle => UnitTypeId::Oracle,
        AbilityId::StargateTrainVoidRay => UnitTypeId::VoidRay,
        AbilityId::StargateTrainPhoenix => UnitTypeId::Phoenix,
        AbilityId::StargateTrainCarrier => UnitTypeId::Carrier,
        _ => UnitTypeId::NotAUnit,
    }
}
