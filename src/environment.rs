use crate::gpr::GPR;

pub type GPRIndex = usize; // index into Environment.gprs

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub map: std::collections::HashMap<std::path::PathBuf, GPRIndex>,
    pub gprs: Vec<Option<GPR>>,
}
