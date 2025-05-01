impl Rebicycler {

    pub fn control_army(&self, army: Units) {
         
}

}


struct ArmyController {
      assignments: HashMap<u64, ArmyAssignment>,

}


struct ArmyAssignment {
    unit: u64,
    assignment: Tactic,
}

enum Tactic {
     AttackMove(Point2),
     StutterMove(Point2),
     DirectMove(Point2),
}