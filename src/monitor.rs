use crate::protoss_bot::ReBiCycler;
use rust_sc2::ids::UpgradeId;
use rust_sc2::{ids::AbilityId, prelude::UnitTypeId, units::Units};
use std::collections::HashMap;
use std::hash::Hash;
impl ReBiCycler {
    const fn monitor(&self) {}

    fn production_tab(&mut self) {
        let data = self.production_facilities();
        let mut lines: Vec<(String, String, String, String)> = Vec::new();
        for ((unit, ability), (count, progress)) in &data {
            let structure_name = crate::building_names(unit);
            let producing = if let Some(a) = ability {
                format!("{:?}", crate::ability_produces(a))
            } else {
                String::new()
            };
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
        let formatted = self.display_production(&mut lines);
        for line in formatted {
            self.display_terminal.write_line_to_pane("Production", line);
        }
    }

    fn display_production(
        &mut self,
        producing: &mut Vec<(String, String, String, String)>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        let same_sep = " - ";
        producing.sort();
        let mut active_structure = String::new();

        while let Some((name, product, count, progress)) = producing.pop() {
            if name != active_structure {
                active_structure = name.clone();
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

    fn idle_tech_structures(&self) -> HashMap<UnitTypeId, usize> {
        let idle_structures = self
            .units
            .my
            .structures
            .idle()
            .filter(|u| crate::is_protoss_tech(u.type_id()));
        Self::count_unit_types(idle_structures)
    }

    fn display_protoss_research(&self) {
        [
            UpgradeId::ProtossGroundWeaponsLevel1,
            UpgradeId::ProtossGroundWeaponsLevel2,
            UpgradeId::ProtossGroundWeaponsLevel3,
        ];
        [
            UpgradeId::ProtossGroundArmorsLevel1,
            UpgradeId::ProtossGroundArmorsLevel2,
            UpgradeId::ProtossGroundArmorsLevel3,
        ];
        [
            UpgradeId::ProtossAirWeaponsLevel1,
            UpgradeId::ProtossAirWeaponsLevel2,
            UpgradeId::ProtossAirWeaponsLevel3,
        ];
        [
            UpgradeId::ProtossAirArmorsLevel1,
            UpgradeId::ProtossAirArmorsLevel2,
            UpgradeId::ProtossAirArmorsLevel3,
        ];
        [
            UpgradeId::ProtossShieldsLevel1,
            UpgradeId::ProtossShieldsLevel2,
            UpgradeId::ProtossShieldsLevel3,
        ];
        let lines: Vec<String> = Vec::new();
        for unit in self
            .units
            .my
            .structures
            .filter(|u| crate::is_protoss_tech(u.type_id()))
        {
            // Ground:
            // 🛡️🛡️🛡️🔪🔪🔪
            // Air:
            // 🛡️🛡️  🔪🔪
            // Shield:
            // 🔵

            // 🌀 👟 💠 💎
            // 🌩️ ​👁️ 🔆 🚀
            // 🧞 🥏 🌡️ 💥

            // In progress:
            // 👁️[56s left]
        }
    }
    fn army_composition(&self) -> HashMap<UnitTypeId, usize> {
        let army = self.units.my.units.filter(|u| !u.is_worker());

        Self::count_unit_types(army)
    }

    fn display_construction(&mut self) {
        let mut out = Vec::new();

        for unit in self
            .units
            .my
            .structures
            .iter()
            .filter(|u| u.build_progress() != 1.0)
        {
            out.push(format!(
                "{:?}: {:.0}%",
                unit.type_id(),
                100.0 * unit.build_progress()
            ));
        }
        for line in out {
            self.display_terminal
                .write_line_to_pane("Construction", line);
        }
    }

    fn count_unit_types(units: Units) -> HashMap<UnitTypeId, usize> {
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
