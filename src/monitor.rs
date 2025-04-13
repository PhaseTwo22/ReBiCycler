use crate::protoss_bot::ReBiCycler;

use itertools::Itertools;
use rust_sc2::ids::{BuffId, UpgradeId};
use rust_sc2::{ids::AbilityId, prelude::UnitTypeId, units::Units};
use std::collections::HashMap;

use std::hash::Hash;

const ARMOR_ICON: &str = "🛡️";
const WEAPONS_ICON: &str = "🔪";
const SHIELD_ICON: &str = "🔵";
const WARPGATE_ICON: &str = "🌀";
const STORM_ICON: &str = "🌩️";
const DT_BLINK_ICON: &str = "🧞";
const CHARGE_ICON: &str = "👟";
const BLINK_ICON: &str = "👁️";
const GLAIVES_ICON: &str = "🥏";
const PRISMSPEED_ICON: &str = "💠";
const OBSERVERSPEED_ICON: &str = "🔆";
const LANCES_ICON: &str = "🌡️";
const PHEONIXRANGE_ICON: &str = "💎";
const VOIDSPEED_ICON: &str = "🚀";
const TECTONIC_ICON: &str = "💥";
const NOT_RESEARCHED: &str = "  ";

impl ReBiCycler {
    pub fn monitor(&mut self, _frame_no: usize) {
        self.display_general();
        self.display_construction();
        self.display_structures();

        self.display_protoss_research();

        self.production_tab();
        self.show_chronos();

        self.army_composition();

        self.display_terminal.flush();
    }

    fn display_general(&mut self) {
        let players = self
            .game_info
            .players
            .values()
            .flat_map(|p| &p.player_name)
            .join(" vs ");
        let map = format!("{} | {}", self.game_info.map_name, players);
        let header = format!(
            "M: {} G: {} S:{}/{}",
            self.minerals, self.vespene, self.supply_used, self.supply_cap
        );
        self.display_terminal.write_line_to_header(map);
        self.display_terminal.write_line_to_header(header);
        self.display_terminal
            .write_line_to_header(format!("{:?}", self.siting_director));
    }

    fn production_tab(&mut self) {
        let data = self.production_facilities();
        let mut lines: Vec<(String, String, String, String)> = Vec::new();
        for ((unit, ability), (count, progress)) in &data {
            let structure_name = crate::building_names(*unit);
            let producing = ability.as_ref().map_or_else(String::new, |a| {
                format!("{:?}", crate::ability_produces(*a))
            });
            let out = (
                structure_name,
                producing,
                count.to_string(),
                if ability.is_none() {
                    String::new()
                } else {
                    format!(": {progress}")
                },
            );
            lines.push(out);
        }
        lines.sort();
        let formatted = Self::format_production(&mut lines);
        for line in formatted {
            self.display_terminal
                .write_line_to_pane("Production", line, false);
        }
    }

    fn show_chronos(&mut self) {
        let mut chronos = Vec::new();
        for (chronoed_unit, (ability, _, _)) in self.units.my.structures.iter().filter_map(|u| {
            if u.has_buff(BuffId::ChronoBoostEnergyCost) && u.order().is_some() {
                Some((u, u.order()?))
            } else {
                None
            }
        }) {
            let out = format!(
                "{:?}:{:?}",
                chronoed_unit.type_id(),
                crate::ability_produces(ability)
            );
            chronos.push(out);
        }

        self.display_terminal
            .write_line_to_pane("Production", "Chrono's:".to_string(), false);
        for line in chronos {
            self.display_terminal
                .write_line_to_pane("Production", line, false);
        }
    }

    fn format_production(producing: &mut Vec<(String, String, String, String)>) -> Vec<String> {
        let mut out = Vec::new();
        let same_sep = " - ";
        producing.sort();
        let mut active_structure = String::new();

        while let Some((name, product, count, progress)) = producing.pop() {
            if name != active_structure {
                active_structure.clone_from(&name);
                out.push(name);
            }
            let line = format!("{same_sep}{product}[{count}]{progress}");
            out.push(line);
        }
        out
    }

    fn production_facilities(&self) -> HashMap<(UnitTypeId, Option<AbilityId>), (usize, f32)> {
        let mut count_and_max: HashMap<(UnitTypeId, Option<AbilityId>), (usize, f32)> =
            HashMap::new();
        for unit in self
            .units
            .my
            .structures
            .iter()
            .filter(|u| crate::is_protoss_production(u.type_id()))
        {
            let (key, progress) = if let Some((ability, _target, progress)) = unit.order() {
                ((unit.type_id(), Some(ability)), progress)
            } else {
                ((unit.type_id(), None), 0.0)
            };

            count_and_max
                .entry(key)
                .and_modify(|(count, current_max)| {
                    *count += 1;
                    *current_max = if *current_max > progress {
                        *current_max
                    } else {
                        progress
                    };
                })
                .or_insert((1, 0.0));
        }
        count_and_max
    }

    fn display_protoss_research(&mut self) {
        let mut lines = Vec::new();
        let mut standard_upgrades = self.get_protoss_standard_upgrades();

        let mut ability_upgrades = self.get_protoss_ability_upgrades();

        lines.append(&mut standard_upgrades);
        lines.append(&mut ability_upgrades);

        for (ability, _target, progress) in self
            .units
            .my
            .structures
            .filter(|u| crate::is_protoss_tech(u.type_id()))
            .iter()
            .filter_map(rust_sc2::prelude::Unit::order)
        {
            let out = format!("- {:?}:{:.2}%", ability, progress * 100.0);
            lines.push(out);
        }

        for line in lines {
            self.display_terminal
                .write_line_to_pane("Research", line, false);
        }
    }

    fn get_protoss_standard_upgrades(&self) -> Vec<String> {
        let ground_weapons = [
            UpgradeId::ProtossGroundWeaponsLevel1,
            UpgradeId::ProtossGroundWeaponsLevel2,
            UpgradeId::ProtossGroundWeaponsLevel3,
        ]
        .map(|u| {
            if self.has_upgrade(u) {
                WEAPONS_ICON
            } else {
                ""
            }
        })
        .join("");

        let ground_armor = [
            UpgradeId::ProtossGroundArmorsLevel1,
            UpgradeId::ProtossGroundArmorsLevel2,
            UpgradeId::ProtossGroundArmorsLevel3,
        ]
        .map(|u| if self.has_upgrade(u) { ARMOR_ICON } else { "" })
        .join("");

        let air_weapons = [
            UpgradeId::ProtossAirWeaponsLevel1,
            UpgradeId::ProtossAirWeaponsLevel2,
            UpgradeId::ProtossAirWeaponsLevel3,
        ]
        .map(|u| {
            if self.has_upgrade(u) {
                WEAPONS_ICON
            } else {
                ""
            }
        })
        .join("");

        let air_armor = [
            UpgradeId::ProtossAirArmorsLevel1,
            UpgradeId::ProtossAirArmorsLevel2,
            UpgradeId::ProtossAirArmorsLevel3,
        ]
        .map(|u| if self.has_upgrade(u) { ARMOR_ICON } else { "" })
        .join("");

        let shields = [
            UpgradeId::ProtossShieldsLevel1,
            UpgradeId::ProtossShieldsLevel2,
            UpgradeId::ProtossShieldsLevel3,
        ]
        .map(|u| if self.has_upgrade(u) { SHIELD_ICON } else { "" })
        .join("");

        vec![
            format!("Ground: {ground_armor}{ground_weapons}"),
            format!("Air: {air_armor}{air_weapons}"),
            format!("Shields: {shields}"),
        ]
    }

    fn get_protoss_ability_upgrades(&self) -> Vec<String> {
        vec![
            [
                (UpgradeId::WarpGateResearch, WARPGATE_ICON),
                (UpgradeId::PsiStormTech, STORM_ICON),
                (UpgradeId::DarkTemplarBlinkUpgrade, DT_BLINK_ICON),
            ],
            [
                (UpgradeId::Charge, CHARGE_ICON),
                (UpgradeId::BlinkTech, BLINK_ICON),
                (UpgradeId::AdeptPiercingAttack, GLAIVES_ICON),
            ],
            [
                (UpgradeId::GraviticDrive, PRISMSPEED_ICON),
                (UpgradeId::ObserverGraviticBooster, OBSERVERSPEED_ICON),
                (UpgradeId::ExtendedThermalLance, LANCES_ICON),
            ],
            [
                (UpgradeId::AnionPulseCrystals, PHEONIXRANGE_ICON),
                (UpgradeId::VoidRaySpeedUpgrade, VOIDSPEED_ICON),
                (UpgradeId::TempestGroundAttackUpgrade, TECTONIC_ICON),
            ],
        ]
        .iter()
        .map(|array| {
            array
                .map(|(up, icon)| {
                    if self.has_upgrade(up) {
                        icon
                    } else {
                        NOT_RESEARCHED
                    }
                })
                .join(" ")
        })
        .collect()
    }

    #[allow(clippy::cast_possible_truncation)]
    fn army_composition(&mut self) {
        let army = self.units.my.units.filter(|u| !u.is_worker());

        let existing_workers = self
            .supply_workers
            .saturating_sub(self.counter().ordered().count(UnitTypeId::Probe) as u32);
        let msg = format!("Workers: {existing_workers}");
        self.display_terminal.write_line_to_pane("Army", msg, false);

        for (unit, count) in Self::count_unit_types(&army) {
            let out = format!("- {unit:?}: {count}");
            self.display_terminal.write_line_to_pane("Army", out, false);
        }
    }

    fn display_construction(&mut self) {
        let mut out = Vec::new();

        for unit in self
            .units
            .my
            .structures
            .iter()
            .filter(|u| u.build_progress() != 1.0_f32)
        {
            out.push(format!(
                "{:?}: {:.0}%",
                unit.type_id(),
                100.0 * unit.build_progress()
            ));
        }
        for line in out {
            self.display_terminal
                .write_line_to_pane("Construction", line, false);
        }
    }

    fn display_structures(&mut self) {
        self.display_terminal
            .write_line_to_pane("Construction", "Finished:".to_string(), false);
        for (unit_type, count) in
            Self::count_unit_types(&self.units.my.structures.filter(|u| u.is_ready()))
        {
            self.display_terminal.write_line_to_pane(
                "Construction",
                format!("{unit_type:?}[{count}]"),
                false,
            );
        }
    }

    fn count_unit_types(units: &Units) -> HashMap<UnitTypeId, usize> {
        let mut counts: HashMap<UnitTypeId, usize> = HashMap::new();
        let _: () = units
            .iter()
            .map(|u| increment_map(&mut counts, u.type_id()))
            .collect();
        counts
    }
}

fn increment_map<T>(map: &mut HashMap<T, usize>, key: T)
where
    T: Hash + Eq,
{
    let new_count = map.get(&key).unwrap_or(&0) + 1;
    map.insert(key, new_count);
}
