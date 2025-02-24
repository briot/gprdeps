use crate::{environment::Environment, errors::Error, settings::Settings};
use std::collections::HashMap;

pub struct ActionDuplicates {}

impl ActionDuplicates {
    /// Look for duplicate filenames.
    /// In general, those create ambiguities, so are better avoided.
    /// However, it is sometimes necessary, for instance when the body of an
    /// Ada unit is implemented in different files depending on the scenario.
    /// This function tries to take that into account to avoid false positives,
    /// by only repeating files that appear together in the same scenario.
    pub fn perform(
        &self,
        env: &Environment,
        settings: &Settings,
    ) -> Result<(), Error> {
        let mut seen = HashMap::new();
        env.graph
            .iter_project_nodes()
            .flat_map(|(gprnode, gprpath)| {
                env.graph
                    .iter_source_nodes_of_project(gprnode)
                    .map(move |path| (gprpath, path))
            })
            .filter(|(_, filepath)| env.files[*filepath].borrow().lang == "ada")
            .for_each(|(gprpath, filepath)| {
                if let Some(simple) = filepath.file_name() {
                    if let Some(base) = simple.to_str() {
                        // Do not report when in same project (we could detect
                        // whether scenarios overlap, but for now this is
                        // detected by the builder)
                        match seen.get(base) {
                            None => {
                                seen.insert(base.to_string(), gprpath);
                            }
                            Some(gpr) => {
                                if *gpr != gprpath {
                                    println!(
                                        "MANU duplicate {} in {} and {}",
                                        base,
                                        settings.display_path(gpr),
                                        settings.display_path(gprpath),
                                    );
                                }
                            }
                        }
                    }
                }
            });

        Ok(())
    }
}
