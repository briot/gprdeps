/// Project data can be varied based on the values of one or more variables.
/// These variables (named "scenario variables") are typed (so can only take
/// a specific set of values), and can be tested in case statements.
/// When we parse project files, we evaluate all scenarios simultaneously.
/// For instance, if we have
///     project A is
///         type Mode_Type  is ("debug", "optimize", "lto");
///         type Check_Type is ("none", "some", "most");
///         Mode  : Mode_Type := external ("MODE");
///         Check : Check_Type := external ("CHECK");
///         case Mode is
///            when "debug" => for Source_Dirs use ("src1/", "src2/");
///            when others  => for Source_Dirs use ("src1", "src3");
///               case Check is
///                  when "most"  => for Source_Dirs use Source_Dirs & "src4";
///                  when others  => null;
///               end case;
///         end case;
///
///         for Excluded_Source_Files use ();  --  implicit in general
///         case Check is
///            when "none" => for Excluded_Source_Files use ("a.ads");
///            when others => null;
///         end case;
///     end A;
///
/// Then internally we create multiple scenarios:
///     s0         => ()
///     s1         => (mode=debug)
///     s2         => (mode=optimize|lto)
///     s3         => (mode=optimize|lto, check=most)
///     s4         => (check=none)
///     s5 = s1|s2 => () = s0                       # for "src1"
///     s6 = s0-s4 => (check=some|most)             # for excluded_source_files
///     s7 = s1*s6 => (mode=debug,check=some|most)  # for source files, later
///     s8 = s1*s4 => (mode=debug,check=none)       # for source files, later
///     s9 = s2*s6 => (mode=optimize|lto,check=some|most)
///     s10= s2*s4 => (mode=optimize|lto,check=none)
///     s11= s3*s6 => (mode=optimize|lto,check=most)
/// And the attributes of the project are parsed as:
///     source_dirs = (s0, "src1"), (s1, "src2"), (s2, "src3"), (s3, "src4")
///     excluded_source_files = (s6, ) (s4, "a.ads")
///
/// If we parse another project, we will create additional scenarios.  Scenarios
/// can overlap (for instance s3 is fully included in s2), but are not
/// duplicated, for efficiency reasons.
///
/// The second stage of processing for projects is to find the list of source
/// files.  For this, we check the files in all directories:
///     s0  src1 => a.ads, b.ads
///     s1  src2 => b.adb
///     s2  src3 => b.adb
///     s3  src4 => c.ads
/// We need to intersect those with the excluded source files attribute, and
/// create additional scenarios:
///     s0*s6=s6    => src1 - ()        => src1/a.ads, src1/b.ads
///     s0*s4=s4    => src1 - ("a.ads") => src1/b.ads
///     s1*s6=s7    => src2 - ()        => src2/b.adb
///     s1*s4=s8    => src2 - ("a.ads") => src2/b.adb
///     s2*s6=s9    => src3 - ()        => src3/b.adb
///     s2*s4=s10   => src3 - ("a.ads") => src3/b.adb
///     s3*s6=s11   => src4 - ()        => src4/c.ads
///     s3*s4=()    => ()
///
/// Now, for instance to find the full list of source files in the scenario
///     s20 => (mode=optimize,check=none)
/// we need to intersect that scenario with each of the ones used in the list of
/// source files, and keep non-empty ones:
///     s20*s6  = empty
///     s20*s4  = not empty    => src1/b.ads
///     s20*s7  = empty
///     s20*s8  = empty
///     s20*s9  = empty
///     s20*s10 = not empty    => src3/b.adb
///     s20*s11 = empty
///
/// Likewise, when we later want to resolve file dependencies (e.g. we have
/// a project B that imports A, and one of its files d.ads depends on
/// b.adb).  We thus take the intersection of each scenario where d.ads exists
/// (say s0 to simplify) which each scenario needed for A's source_files
/// attribute, to know which b.adb gets used.
///     s0*s7  = s7  => src2/b.adb
///     s0*s8  = s8  => src2/b.adb
///     s0*s9  = s9  => src3/b.adb
///     s0*s10 = s10 => src3/b.adb
/// There are duplicates here, so we can group things to reduce the size.
///     s7|s8  = (mode=debug,check=some|most) | (mode=debug,check=none)
///            = (mode=debug) = s1     => src2/b.adb
///     s9|s10 = (mode=opt|lto,check=some|most) | (mode=opt|lto,check=none)
///            = (mode=opt|lto) = s2   => src3/b.adb
use crate::scenario_variables::ScenarioVariable;

pub struct Scenario(u16);

#[derive(Default)]
pub struct AllScenarios {
    variables: std::collections::HashMap<String, ScenarioVariable>,
}

impl AllScenarios {
    /// Declares a new scenario variables and the list of all values it can
    /// accept.  If the variable is already declared, check that we are
    /// declaring the same set of values
    pub fn try_add_variable(&mut self, name: &str, valid: Vec<&str>) -> Result<(), String> {
        match self.variables.get(name) {
            None => {
                println!("MANU found type {:?} {:?}", name, valid);
                self.variables
                    .insert(name.to_owned(), ScenarioVariable::new(name, valid));
                Ok(())
            }
            Some(oldvar) => {
                if oldvar.has_same_valid(&valid) {
                    Ok(())
                } else {
                    Err(format!(
                        "Variable {} already defined with another set of \
                        values (was {:?}, now {:?})",
                        name,
                        oldvar.list_valid(),
                        valid.join(", "),
                    )
                    .to_owned())
                }
            }
        }
    }
}

// impl Default for All_Scenarios {
//     fn default() -> Self {
//         All_Scenarios {
//             variables: vec![],
//         }
//     }
// }
