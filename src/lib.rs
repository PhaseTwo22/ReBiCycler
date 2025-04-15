use rust_sc2::prelude::*;
use std::{collections::HashMap, fmt::Debug, hash::Hash};
mod base_manager;
mod build_order_manager;
mod build_orders;
mod construction;
mod errors;
mod knowledge;
mod map_viz;
mod micro;
mod monitor;
pub mod protoss_bot;
mod readout;
mod siting;

pub const PYLON_POWER_RADIUS: f32 = 6.5;
pub const PRISM_POWER_RADIUS: f32 = 3.75;

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
    pub tag: u64,
    pub unit_type: UnitTypeId,
}
impl Tag {
    #[must_use]
    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            tag: unit.tag(),
            unit_type: unit.type_id(),
        }
    }
    #[must_use]
    pub const fn default() -> Self {
        Self {
            tag: 0,
            unit_type: UnitTypeId::NotAUnit,
        }
    }
}
#[must_use]
pub const fn is_protoss_building(unit: &UnitTypeId) -> bool {
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
pub const fn is_minerals(unit: UnitTypeId) -> bool {
    use UnitTypeId as U;
    matches!(
        unit,
        U::MineralField
            | U::MineralField750
            | U::MineralField450
            | U::LabMineralField
            | U::LabMineralField750
            | U::RichMineralField
            | U::RichMineralField750
            | U::MineralFieldOpaque
            | U::MineralFieldOpaque900
            | U::PurifierMineralField
            | U::PurifierMineralField750
            | U::PurifierRichMineralField
            | U::PurifierRichMineralField750
            | U::BattleStationMineralField
            | U::BattleStationMineralField750
    )
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

#[must_use]
pub const fn structure_needs_power(unit: &UnitTypeId) -> bool {
    if is_protoss_building(unit) {
        !matches!(
            unit,
            UnitTypeId::Assimilator
                | UnitTypeId::AssimilatorRich
                | UnitTypeId::Nexus
                | UnitTypeId::Pylon
        )
    } else {
        true
    }
}

fn building_names(unit: UnitTypeId) -> String {
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

const fn ability_produces(ability: AbilityId) -> UnitTypeId {
    match ability {
        AbilityId::NexusTrainProbe => UnitTypeId::Probe,
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

#[must_use]
pub fn count_unit_types(units: &Units) -> HashMap<UnitTypeId, usize> {
    let mut counts: HashMap<UnitTypeId, usize> = HashMap::new();
    let _: () = units
        .iter()
        .map(|u| increment_map(&mut counts, u.type_id()))
        .collect();
    counts
}

fn increment_map<T>(map: &mut HashMap<T, usize>, key: T)
where
    T: Hash + Eq,
{
    let new_count = map.get(&key).unwrap_or(&0) + 1;
    map.insert(key, new_count);
}

#[must_use] pub fn closeratest(anchor: Point2, p1: Point2, p2: Point2) -> std::cmp::Ordering {
    let d2p1 = anchor.distance(p1);
    let d2p2 = anchor.distance(p2);
    d2p1.total_cmp(&d2p2)
}
