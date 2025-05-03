use std::fmt::Display;

use rust_sc2::prelude::{AbilityId, UnitTypeId, UpgradeId};

use crate::chatter::ChatAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildCondition {
    SupplyAtLeast(u32),
    SupplyBetween(u32, u32),
    SupplyLeftBelow(u32),
    TechComplete(UpgradeId),
    StructureComplete(UnitTypeId),
    LessThanCount(UnitTypeId, usize),
    AtLeastCount(UnitTypeId, usize),
    DontHaveAnyDone(UnitTypeId),
    DontHaveAnyStarted(UnitTypeId),
    Always,
    Never,
    TotalAndOrderedAtLeast(UnitTypeId, usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
    Chrono(AbilityId),
    ChronoWhatever(UnitTypeId),
    Research(UpgradeId, AbilityId, UnitTypeId),
    Expand,
    Chat(ChatAction),
    Surrender,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentState {
    NotYetStarted,
    Active,
    Completed,
    Restricted,
}

impl Display for ComponentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            Self::NotYetStarted => "➖",
            Self::Active => "⏳",
            Self::Completed => "✅",
            Self::Restricted => "❌",
        }
        .to_string();
        write!(f, "{out}")
    }
}

#[cfg(test)]
mod tests {}
