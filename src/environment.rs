use crate::gpr::GPR;
use crate::scenarios::AllScenarios;

#[derive(Clone, Copy)]
pub struct GPRIndex(pub usize);  // index into Environment.gprs

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub map: std::collections::HashMap<std::path::PathBuf, GPRIndex>,
    pub gprs: Vec<Option<GPR>>,

    pub scenarios: AllScenarios,
}
