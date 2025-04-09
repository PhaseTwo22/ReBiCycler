impl ReBiCycler {
fn monitor(&self) {

}

fn production_tab (&self) {
let data = self.production_facilities();
for ((unit, ability),(count,progress))in data.iter() {
    let producing = if let Some(name) = ability {crate::ability_produces(ability)} else{""};
}
}

fn idle_production_facilities(&self) {
let idle_structures = self.units.my.structures.idle().filter(|u|crate::is_protoss_production(u.type_id());
count_unit_types(idle_structures)
}

fn production_facilities(&self) {
let count_and_max : HashMap<(UnitTypeId, Option(AbilityId)), (usize, f32)> = HashMap::new();
for unit in  self.units.my.structures.iter().filter(|u|crate::is_protoss_production(u.type_id())){
    let (key, progress) = if let Some((ability, _target, progress)) = unit.orders() {
((unit.type_id(), ability), progress)
})
else ((unit.type_id(), None), 0.0)
};
    count_and_max.entry(key).and_modify(|(count, max)| (count + 1, if max > progress {max} else {progress}).or_insert((1,0.0));

}
count_and_max
}


fn idle_tech_structures(&self) {
let idle_structures = self.units.my.structures.idle().filter(|u|crate::is_protoss_tech(u.type_id());
count_unit_types(idle_structures)
}

fn army_composition(&self) {
let mut count:HashMap<UnitTypeId, usize>
let _ : ()= self.units.my.units.iter().filter(|u| !u.is_worker()).map( |u| {
increment_map(&mut count, u.type_id())
}
).collect();
count
}

}


fn count_unit_types(units: Units) -> HashMap<UnitTypeId, usize> {
let mut counts : HashMap<UnitTypeId, usize> = HashMap::new();
let _ : () = units.iter().map(|u| {
increment_map(&mut counts, u.type_id())
}).collect();
counts
}



fn increment_map<T>(map:&mut HashMap<T,usize>, key:  T) {
let new_count = map.get(key).unwrap_or(0) + 1;
map.insert(key, new_count);
}
