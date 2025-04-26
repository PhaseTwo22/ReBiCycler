use std::fmt::Display;

use rust_sc2::prelude::{AbilityId, UnitTypeId, UpgradeId};

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
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
    Chrono(AbilityId),
    ChronoWhatever(UnitTypeId),
    Research(UpgradeId, AbilityId, UnitTypeId),
    Expand,
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildOrderComponent {
    pub name: String,
    pub start_conditions: Vec<BuildCondition>,
    pub end_conditions: Vec<BuildCondition>,
    pub action: BuildOrderAction,
    pub state: ComponentState,
}

impl Display for BuildOrderComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.state)
    }
}
impl BuildOrderComponent {
    pub fn new(
        name: &str,
        start_conditions: &[BuildCondition],
        end_conditions: &[BuildCondition],
        action: BuildOrderAction,
    ) -> Self {
        Self {
            name: name.to_string(),
            start_conditions: start_conditions.into(),
            end_conditions: end_conditions.into(),
            action,
            state: ComponentState::NotYetStarted,
        }
    }
}

#[cfg(test)]
mod tests {
    
}
