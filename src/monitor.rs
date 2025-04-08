impl ReBiCycler {
fn monitor(&self) {

}

fn production_tab (&self) {

}

fn idle_production_facilities(&self) {
let idle_structures = self.units.my.structures.idle().filter(|u|crate::is_protoss_production(u.type_id());
count_unit_types(idle_structures)
}

fn busy_facilities(&self) {
let count_and_max : HashMap<(UnitTypeId, AbilityId), (usize, f32)> = HashMap::new();
let busy = self.units.my.structures.iter().filter_map(|u| if u.order().is_some() {Some((u.type_id(), u.order())} else {None});

for (unit_type, (ability, _target, progress)) in busy {
let (count, top_progress) = count_and_max.get((unit_type, ability)).unwrap_or(0,0.0);
count_and_max.insert((unit_type,ability),(count + 1, f32::max(progress, top_progress)));
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
