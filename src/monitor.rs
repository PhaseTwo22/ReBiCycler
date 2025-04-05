impl ReBiCycler {
fn monitor(&self) {

}

fn idle_facilities(&self) {
let idle_structures = self.units.my.structures.idle();
count_unit_types(idle_structures)
}

fn busy_facilities(&self) {
let count_and_max : HashMap<AbilityId, (usize, f32)> = HashMap::new();
let busy = self.units.my.structures.iter().filter_map(|u| u.order());

for (ability, _target, progress) in busy {
let (count, top_progress) = count_and_max.get(ability).unwrap_or(0,0.0);
count_and_max.insert(ability,(count + 1, f32::max(progress, top_progress)));
}
count_and_max
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
