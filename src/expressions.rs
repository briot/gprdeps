use crate::scenarios::Scenario;

pub struct VarValue {
    name: String,
    value: Option<String>, // If None, all values allowed
}

pub struct Expression<'a, T> {
    value: Vec<(&'a Scenario, T)>,
}

impl<'a, T> Expression<'a, T> {
    pub fn get(&self, scenario: &Scenario) -> Option<T> {
        None
    }
}
