use std::fmt::Debug;

pub struct UnitEmploymentError(pub String);
impl Debug for UnitEmploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in employment: {}", self.0)
    }
}

pub struct InvalidUnitError(pub String);
impl Debug for InvalidUnitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad unit: {}", self.0)
    }
}
#[derive(Debug)]
pub struct DataError(pub String);
