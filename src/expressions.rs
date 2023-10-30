use crate::scenarios::Scenario;

pub struct VarValue {
    _name: String,
    _value: Option<String>, // If None, all values allowed
}

pub struct Expression<'a, T> {
    _value: Vec<(&'a Scenario, T)>,
}

impl<'a, T> Expression<'a, T> {
    pub fn get(&self, _scenario: &Scenario) -> Option<T> {
        None
    }
}
