use std::collections::HashMap;

use rust_sc2::prelude::*;

use crate::Tag;

pub struct Platoon {
    pub state: PlatoonState,
    pub squads: Vec<Squad>,
}

pub struct Squad {
    pub squad_type: SquadType,
    pub state: SquadMicroState,
    pub members: Vec<MicroTag>,
}

impl Squad {
    pub fn new(squad_type: SquadType) -> Self {
        Self {
            squad_type,
            state: SquadMicroState::Idle,
            members: Vec::new(),
        }
    }

    pub fn add_unit(&mut self, unit: MicroTag) {
        self.members.push(unit);
    }

    pub fn remove_unit(&mut self, unit: Tag) -> Option<Tag> {
        if let Some(tag) = self.members.iter().find(|u| u.tag == unit) {
            let out = tag.tag;
            self.members.retain(|x| x.tag != unit);
            return Some(out);
        }
        None
    }

    pub fn disband(self) -> Vec<Tag> {
        self.members.into_iter().map(|u| u.tag).collect()
    }
}

pub enum SquadType {
    MineralMiners,
    GasMiners,
}

pub enum PlatoonState {
    Nominal,
    Retreat,
    ControlRegion,
    Assault,
}

pub enum SquadMicroState {
    Idle,
    AMoveForAiur,
    StutterForAiur,
    StutterSkirmish,
    MineMinerals,
    MineGas,
    Regroup,
    Retreat,
    Rally,
    KillCreep,
}

pub struct MicroTag {
    tag: Tag,
    micro_state: UnitMicroState,
}
impl PartialEq for MicroTag {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }

    fn ne(&self, other: &Self) -> bool {
        self.tag != other.tag
    }
}

pub enum UnitMicroState {
    AMove,
    GatherMove(Point2),
    Gather(u64),
    ReturnMove(Point2),
    ReturnCargo,
    Retreat,
    Rally,
    HardMove,
}

pub struct MicroBaseManager {
    nexus: Tag,
    location: Point2,
    mineral_squads: HashMap<u64, Squad>,
    gas_squads: HashMap<u64, Squad>,
}

impl MicroBaseManager {
    pub fn new(nexus: Tag, location: Point2, minerals: Vec<u64>, assimilators: Vec<u64>) -> Self {
        Self {
            nexus,
            location,
            mineral_squads: minerals
                .into_iter()
                .map(|m| (m, Squad::new(SquadType::MineralMiners)))
                .collect(),
            gas_squads: assimilators
                .into_iter()
                .map(|gl| (gl, Squad::new(SquadType::GasMiners)))
                .collect(),
        }
    }

    pub fn add_assimilator(&mut self, tag: u64) {
        self.gas_squads
            .insert(tag, Squad::new(SquadType::GasMiners));
        if self.gas_squads.len() > 2 {
            println!("We have more than 2 gasses at this base: {:?}", self.nexus);
        }
    }

    pub fn dismiss_assimilator(&mut self, tag: Tag) -> Option<Vec<Tag>> {
        let squad = self.gas_squads.remove(&tag.tag)?;
        Some(squad.disband())
    }

    pub fn dismiss_unit(&mut self, tag: Tag) -> Option<Vec<Tag>> {
        match tag.type_id {
            UnitTypeId::Assimilator | UnitTypeId::AssimilatorRich => self.dismiss_assimilator(tag),
            UnitTypeId::Probe => self.dismiss_probe(tag),
            _ => {
                println!("i dunno how to dismiss {tag:?}");
                None
            }
        }
    }

    fn dismiss_probe(&mut self, tag: Tag) -> Option<Vec<Tag>> {
        if let Some(probe) = self
            .gas_squads
            .values_mut()
            .find_map(|gs| gs.remove_unit(tag))
        {
            Some(vec![probe])
        } else {
            self.mineral_squads
                .values_mut()
                .find_map(|gs| gs.remove_unit(tag))
                .map(|probe| vec![probe])
        }
    }

    pub fn disband(self) -> Vec<Tag> {
        let mut relinquished_units = Vec::new();

        relinquished_units.append(
            &mut self
                .gas_squads
                .into_values()
                .flat_map(|s| s.disband())
                .collect(),
        );
        relinquished_units.append(
            &mut self
                .mineral_squads
                .into_values()
                .flat_map(|s| s.disband())
                .collect(),
        );

        relinquished_units
    }
}
