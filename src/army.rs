impl Rebicycler {

    pub fn plan_army(&mut self, army: Units) {
         
}

    pub fn command_army(&self)
{

}

    fn command_unit(&self, UnitState) 
{

}
}

struct ArmyController {
      assignments: HashMap<u64, ArmyAssignment>,

}


struct ArmyAssignment {
    unit: UnitState,
    assignment: Tactic,
}

enum Tactic {
     AttackMove(Point2),
     StutterMove(Point2),
     DirectMove(Point2),
}

struct UnitState {
     tag:u64,
     type_id: UnitTypeId,
     vitals : Vitals,
     weapon_cooldown : Option<(f32,f32)>,
     energy: Option<f32>,
}