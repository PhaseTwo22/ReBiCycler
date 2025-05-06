/// we want to make things have strong types, so we can use compile time checks to ensure everything is good. 

/// we can maybe pass stuff around much easier with that too. 

#[derive(Clone)]
struct StrongBase {
    tag:u64,
    vitals: UnitVitals,
    location: Point2,
    is_mine: bool,
}
struct UnitVitals {
    health: Option<u32>,
    max_health: Option<u32>,
    shields: Option<u32>,
    max_shields: Option<u32>,
    energy: Option<u32>,
    max_energy: Option<u32>,
}

struct Zealot {
    base: StrongBase,
    charge_cooldown: Option<f32>,
}

struct Probe {
    base:StrongBase,
    is_holding: HoldingResource,
}

struct MineralField {
    base: StrongBase,
    minerals_left: u32,
    is_gold: bool,
}

struct GasBuilding {
    base: StrongBase,
    gas_left: u32
    is_rich: book,
}
enum HoldingResource {
    None,
    Gas,
    Minerals,
}



trait Strong: Clone {
    fn from_unit<T>(unit:&Unit) -> Result<T,()>;
    fn type_id(&self) -> UnitTypeId;
    fn update(self, unit: &Unit) -> Result<T, ()>;
    
}

trait Loads: Strong {
    fn passengers(&self) -> Vec<impl Strong>;
    th passengers<T>(&Self) -> Vec<T>
where T: Strong;
}