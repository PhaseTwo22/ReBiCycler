use rust_sc2::{ids::UnitTypeId, prelude::Point2, unit::Unit};

use crate::assignment_manager::Identity;

/// we want to make things have strong types, so we can use compile time checks to ensure everything is good.

/// we can maybe pass stuff around much easier with that too.

#[derive(Clone)]
struct StrongBase {
    tag: u64,
    vitals: UnitVitals,
    location: Point2,
    is_mine: bool,
}

#[derive(Clone)]
struct UnitVitals {
    health: Option<u32>,
    max_health: Option<u32>,
    shields: Option<u32>,
    max_shields: Option<u32>,
    energy: Option<u32>,
    max_energy: Option<u32>,
}

#[derive(Clone)]
pub struct Zealot {
    base: StrongBase,
    charge_cooldown: Option<f32>,
}
impl Identity<u64> for Zealot {
    fn id(&self) -> u64 {
        self.base.tag
    }
}
impl Strong for Zealot {
    fn from_unit(unit: &Unit) -> Result<Self, ()> {
        match unit.type_id() {
            UnitTypeId::Zealot => Ok(todo!()),
            _ => Err(()),
        }
    }
    fn type_id(&self) -> UnitTypeId {
        UnitTypeId::Zealot
    }

    fn update(self, unit: &Unit) -> Result<Self::Output, ()> {
        todo!()
    }

    type Output = Self;
}

struct Probe {
    base: StrongBase,
    is_holding: HoldingResource,
}

struct MineralField {
    base: StrongBase,
    minerals_left: u32,
    is_gold: bool,
}

struct GasBuilding {
    base: StrongBase,
    gas_left: u32,
    is_rich: bool,
}
enum HoldingResource {
    None,
    Gas,
    Minerals,
}

trait Strong: Clone {
    type Output;
    fn from_unit(unit: &Unit) -> Result<Self::Output, ()>;
    fn type_id(&self) -> UnitTypeId;
    fn update(self, unit: &Unit) -> Result<Self::Output, ()>;
}

trait Loads: Strong {
    fn passengers(&self) -> Vec<impl Strong>;
    fn passengers_of_type<T>(&self) -> Vec<T>
    where
        T: Strong;
}
