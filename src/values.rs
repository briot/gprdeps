use crate::gpr::GPR;
use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
use crate::scenarios::{AllScenarios, Scenario};
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum ExprValue {
    Str(HashMap<Scenario, String>),
    StrList(HashMap<Scenario, Vec<String>>),
    PathList(HashMap<Scenario, Vec<std::path::PathBuf>>),
}

impl ExprValue {
    /// An expression that always has the same static value for all scenarios
    pub fn new_with_str(s: &str) -> Self {
        ExprValue::new_with_str_and_scenario(s, Scenario::default())
    }

    pub fn new_with_str_and_scenario(s: &str, scenario: Scenario) -> Self {
        let mut m = HashMap::new();
        m.insert(scenario, s.to_string());
        ExprValue::Str(m)
    }

    // An expression value created as an empty list
    pub fn new_with_list(list: &[&str]) -> Self {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            list.iter().map(|s| s.to_string()).collect(),
        );

        ExprValue::StrList(m)
    }

    /// Given a type declaration (which cannot be declared in case statements,
    /// so has only one set of possible values), generate an expression where
    /// each value is in its own scenario.
    /// For instance, given:
    ///     type Mode_Type is ("debug", "optimize")
    /// and calling this function for the variable "MODE", we create the
    /// following expression:
    ///     MODE=debug    => "debug"
    ///     MODE=optimize => "optimize"
    /// This is used to get the possible values of scenario variables
    pub fn new_with_variable(
        scenarios: &mut AllScenarios,
        varname: &str,
        type_values: &ExprValue,
    ) -> Self {
        let valid = type_values.as_list(); // panic if not a single list
        let mut m = HashMap::new();
        let s0 = Scenario::default();
        for v in valid {
            let s1 = scenarios.split(s0, varname, &[&v]);
            m.insert(s1, v.clone());
        }
        ExprValue::Str(m)
    }

    /// Assumes the expression is a static string valid for all scenarios and
    /// return it.
    pub fn as_string(&self) -> &String {
        match self {
            ExprValue::Str(s) => {
                if s.len() != 1 {
                    panic!("Expected no variants {:?}", self);
                }
                &s[&Scenario::default()]
            }
            _ => panic!("Expected a string {:?}", self),
        }
    }

    /// The expression is assumed to have a single value, for the default
    /// scenario (think of types).  Return that value.
    /// Otherwise panic
    pub fn as_list(&self) -> &Vec<String> {
        match self {
            ExprValue::StrList(v) => &v[&Scenario::default()],
            _ => panic!("Expected a list {:?}", self),
        }
    }

    /// List all scenarios that have an impact on the variable's value
    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
        match self {
            ExprValue::Str(v) => useful.extend(v.keys()),
            ExprValue::StrList(v) => useful.extend(v.keys()),
            ExprValue::PathList(v) => useful.extend(v.keys()),
        }
    }

    /// Given a scenario variable, as setup via new_with_variable, prepares
    /// a mapping from one string value to the corresponding scenario.
    pub fn prepare_case_stmt(
        &self,
    ) -> Result<HashMap<String, Scenario>, String> {
        match self {
            ExprValue::Str(per_scenario) => {
                let mut result = HashMap::new();
                for (s, v) in per_scenario {
                    result.insert(v.clone(), *s);
                }
                Ok(result)
            }
            _ => Err(format!(
                "Variable in a case statement must be a string {:?}",
                self
            )),
        }
    }

    /// Evaluate a raw expression into its final value.
    /// The expression is initially seen in the context of one scenario (matching
    /// the case and when clauses), but its final value might be split into
    /// several scenarios if it is referencing another variable.
    pub fn new_with_raw(
        expr: &RawExpr,
        gpr: &GPR, //  what project what this expression read in ?
        gpr_deps: &[&GPR],
        scenars: &mut AllScenarios,
        scenar: Scenario,
        current_pkg: PackageName,
    ) -> Result<Self, String> {
        match expr {
            RawExpr::Empty => {
                Err(format!("{}: cannot evaluate empty expr", gpr))
            }
            RawExpr::Others => {
                Err(format!("{}: cannot evaluate `others` expr", gpr))
            }
            RawExpr::FuncCall((
                QualifiedName {
                    project: None,
                    package: PackageName::None,
                    name: SimpleName::Name(n),
                },
                args,
            )) => match n.as_ref() {
                "external" => {
                    let varname = match &args[0] {
                        RawExpr::StaticString(v) => v,
                        _ => panic!(
                            "Expected static string for variable \
                                     name in {:?}",
                            expr
                        ),
                    };
                    let default = match args.get(1) {
                        None => ExprValue::new_with_str(""),
                        Some(expr) => ExprValue::new_with_raw(
                            expr,
                            gpr,
                            gpr_deps,
                            scenars,
                            scenar,
                            current_pkg,
                        )?,
                    };
                    match &std::env::var(varname) {
                        Ok(v) => Ok(ExprValue::new_with_str(v)),
                        Err(_) => Ok(default),
                    }
                }
                _ => panic!("{}: Unknown function {:?}", gpr, expr),
            },
            RawExpr::FuncCall(_) => {
                Err(format!("{}: unknown function call {:?}", gpr, expr))
            }
            RawExpr::Name(q) => {
                Ok(gpr.lookup(q, gpr_deps, current_pkg)?.clone())
            }
            RawExpr::StaticString(s) => {
                Ok(ExprValue::new_with_str_and_scenario(s, scenar))
            }
            RawExpr::List(ls) => {
                let mut m: HashMap<Scenario, Vec<String>> = HashMap::new();
                m.insert(scenar, vec![]);
                for expr in ls {
                    // Each element of the list is an expression, which could
                    // have a different value for each scenario.
                    let s = ExprValue::new_with_raw(
                        expr,
                        gpr,
                        gpr_deps,
                        scenars,
                        scenar,
                        current_pkg,
                    )?;
                    match s {
                        ExprValue::Str(per_scenario) => {
                            // We have an existing map:
                            //    s0=[all scenarios] => ["a", "b"]
                            // and want to add a new value, for which we assume
                            // that s1+s2 is the whole world (i.e. s0)
                            //    s1 => "c",
                            //    s2 => "d"
                            // The result is a multi-valued list:
                            //    s0*s1 => ["a", "b", "c"]
                            //    s0*s2 => ["a", "b", "d"]

                            let mut new_m = HashMap::new();
                            for (s2, v2) in per_scenario {
                                for (s1, v1) in &m {
                                    let mut v = v1.clone();
                                    v.push(v2.clone());
                                    new_m.insert(
                                        scenars.intersection(*s1, s2)?,
                                        v,
                                    );
                                }
                            }
                            m = new_m;
                        }
                        _ => Err(format!(
                            "{}: lists can only contain strings",
                            gpr
                        ))?,
                    }
                }
                Ok(ExprValue::StrList(m))
            }
            RawExpr::Ampersand((left, right)) => {
                let l_eval = ExprValue::new_with_raw(
                    left,
                    gpr,
                    gpr_deps,
                    scenars,
                    scenar,
                    current_pkg,
                )?;
                let r_eval = ExprValue::new_with_raw(
                    right,
                    gpr,
                    gpr_deps,
                    scenars,
                    scenar,
                    current_pkg,
                )?;

                match (l_eval, r_eval) {
                    (ExprValue::Str(ls), ExprValue::Str(rs)) => {
                        let mut m = HashMap::new();
                        for (s1, v1) in ls {
                            for (s2, v2) in &rs {
                                // The string v1&v2 is only meaningful for the
                                // mode that is the intersection of s1 and s2.
                                // In other modes, v1 or v2 are considered the
                                // empty string.
                                let mut res = v1.clone();
                                res.push_str(v2);
                                m.insert(scenars.intersection(s1, *s2)?, res);
                            }
                        }
                        Ok(ExprValue::Str(m))
                    }

                    (ExprValue::Str(_), _) => Err(format!(
                        "{}: cannot concatenate string and list",
                        gpr
                    )),

                    (ExprValue::StrList(ls), ExprValue::Str(rs)) => {
                        let mut m = HashMap::new();
                        for (s1, v1) in ls {
                            for (s2, v2) in &rs {
                                let mut res = v1.clone();
                                res.push(v2.clone());
                                m.insert(scenars.intersection(s1, *s2)?, res);
                            }
                        }
                        Ok(ExprValue::StrList(m))
                    }

                    (ExprValue::StrList(ls), ExprValue::StrList(rs)) => {
                        let mut m = HashMap::new();
                        for (s1, v1) in ls {
                            for (s2, v2) in &rs {
                                let mut res = v1.clone();
                                res.extend(v2.clone());
                                m.insert(scenars.intersection(s1, *s2)?, res);
                            }
                        }
                        Ok(ExprValue::StrList(m))
                    }

                    _ => Err(format!("{}: wrong use of &", gpr)),
                }
            }
        }
    }

    /// Find whether (scenar, value) can be merged with any existing state
    /// in self.
    fn merge_internal<T>(
        v_self: &mut HashMap<Scenario, T>,
        v_right: HashMap<Scenario, T>,
        scenars: &mut AllScenarios,
    ) -> Result<(), String>
    where
        T: Eq + std::fmt::Debug,
    {
        for (s2, v2) in v_right {
            let mut merged: Option<(Scenario, Scenario)> = None;
            for (s1, v1) in v_self.iter() {
                if *v1 == v2 {
                    //  Same value in two scenarios ?
                    if let Some(new_s) = scenars.union(*s1, s2) {
                        merged = Some((*s1, new_s));
                        break;
                    }
                }
            }

            match merged {
                None => {
                    if v_self.contains_key(&s2) {
                        Err(format!(
                            "Cannot merge two values, the same scenario occurs \
                             twice {}: {:?} {:?}",
                            s2, v_self, v2))?;
                    }
                    v_self.insert(s2, v2);
                }
                Some((s1, new_s)) => {
                    let old = v_self.remove(&s1);
                    v_self.insert(new_s, old.unwrap());
                }
            }
        }
        Ok(())
    }

    /// Merge two expression values.
    /// There must not be any conflicts (value set for the same scenario in
    /// both self and right, even if the values match).
    pub fn merge(
        &mut self,
        right: ExprValue,
        scenars: &mut AllScenarios,
    ) -> Result<(), String> {
        match (self, right) {
            (ExprValue::Str(v_self), ExprValue::Str(v_right)) => {
                ExprValue::merge_internal(v_self, v_right, scenars)
            }
            (ExprValue::StrList(v_self), ExprValue::StrList(v_right)) => {
                ExprValue::merge_internal(v_self, v_right, scenars)
            }
            (ExprValue::PathList(v_self), ExprValue::PathList(v_right)) => {
                ExprValue::merge_internal(v_self, v_right, scenars)
            }
            (s, r) => Err(format!(
                "values do not have the same type {:?} and {:?}",
                s, r
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gpr::GPR;
    use crate::graph::NodeIndex;
    use crate::rawexpr::tests::{build_expr_list, build_expr_str};
    use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
    use crate::scenarios::{AllScenarios, Scenario};
    use crate::values::ExprValue;
    use std::collections::HashMap;

    #[test]
    fn test_eval() -> Result<(), String> {
        let mut gpr =
            GPR::new(std::path::Path::new("/"), NodeIndex::default(), "dummy");
        let mut scenars = AllScenarios::default();
        let scenar = Scenario::default();
        let pkg = PackageName::None;

        // Evaluate a string
        let expr1 = build_expr_str("value");
        assert_eq!(
            ExprValue::new_with_raw(
                &expr1,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_str("value"),
        );

        // Concatenate two strings
        let expr2 = build_expr_str("value").ampersand(build_expr_str("suffix"));
        assert_eq!(
            ExprValue::new_with_raw(
                &expr2,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_str("valuesuffix"),
        );

        // Evaluate a list of strings
        let expr3 = build_expr_list(&["val1", "val2"]);
        assert_eq!(
            ExprValue::new_with_raw(
                &expr3,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_list(&["val1", "val2"])
        );

        // Evaluate a list of expressions
        let expr4 = RawExpr::List(vec![
            Box::new(
                build_expr_str("value").ampersand(build_expr_str("suffix")),
            ),
            Box::new(build_expr_str("val2")),
        ]);
        assert_eq!(
            ExprValue::new_with_raw(
                &expr4,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_list(&["valuesuffix", "val2"]),
        );

        // Concatenate list and string
        let expr4 = build_expr_list(&["val1", "val2"])
            .ampersand(build_expr_str("value"));
        assert_eq!(
            ExprValue::new_with_raw(
                &expr4,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_list(&["val1", "val2", "value"]),
        );

        // Concatenate two lists
        let expr5 = build_expr_list(&["val1", "val2"])
            .ampersand(build_expr_list(&["val3", "val4"]));
        assert_eq!(
            ExprValue::new_with_raw(
                &expr5,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_list(&["val1", "val2", "val3", "val4"]),
        );

        // Evaluate a qualified name

        gpr.declare(
            PackageName::None,
            SimpleName::Name("var1".to_string()),
            ExprValue::new_with_str("val1"),
        )?;

        let expr =
            build_expr_str("value").ampersand(RawExpr::Name(QualifiedName {
                project: None,
                package: PackageName::None,
                name: SimpleName::Name("var1".to_string()),
            }));
        assert_eq!(
            ExprValue::new_with_raw(
                &expr,
                &gpr,
                &[],
                &mut scenars,
                scenar,
                pkg
            )?,
            ExprValue::new_with_str("valueval1"),
        );

        Ok(())
    }

    #[test]
    fn test_eval_scenario() -> Result<(), String> {
        let mut gpr =
            GPR::new(std::path::Path::new("/"), NodeIndex::default(), "dummy");
        let mut scenars = AllScenarios::default();
        scenars.try_add_variable("MODE", &["debug", "optimize", "lto"])?;
        scenars.try_add_variable("CHECK", &["none", "some", "most"])?;
        let pkg = PackageName::None;
        let s0 = Scenario::default();
        let s2 = scenars.split(s0, "MODE", &["debug", "optimize"]);
        let s3 = scenars.split(s0, "MODE", &["lto"]);
        let s4 = scenars.split(s0, "CHECK", &["some"]);
        let s5 = scenars.split(s0, "CHECK", &["most", "none"]);

        // Assume a variable has different values in two modes
        //     s2=[MODE=debug|optimize]      => "val2"
        //     s3=[MODE=lto]                 => "val3"
        let mut var1 = ExprValue::new_with_raw(
            &build_expr_str("val2"),
            &gpr,
            &[],
            &mut scenars,
            s2,
            pkg,
        )?;
        var1.merge(
            ExprValue::new_with_raw(
                &build_expr_str("val3"),
                &gpr,
                &[],
                &mut scenars,
                s3,
                pkg,
            )?,
            &mut scenars,
        )?;
        gpr.declare(
            PackageName::None,
            SimpleName::Name("var1".to_string()),
            var1,
        )?;

        // Another variable has different values in two modes
        //     s4=[CHECK=some]      => "val4"
        //     s5=[CHECK=most|none] => "val5"
        let mut var2 = ExprValue::new_with_raw(
            &build_expr_str("val4"),
            &gpr,
            &[],
            &mut scenars,
            s4,
            pkg,
        )?;
        var2.merge(
            ExprValue::new_with_raw(
                &build_expr_str("val5"),
                &gpr,
                &[],
                &mut scenars,
                s5,
                pkg,
            )?,
            &mut scenars,
        )?;

        gpr.declare(
            PackageName::None,
            SimpleName::Name("var2".to_string()),
            var2,
        )?;

        // Computing the concatenation results in multiple possible values
        //   s2*s4=s7=[MODE=debug|optimize, CHECK=some]      => "val2val4"
        //   s2*s5=s8=[MODE=debug|optimize, CHECK=most|none] => "val2val5"
        //   s3*s4=s5=[MODE=lto,            CHECK=some]      => "val3val4"
        //   s3*s4=s6=[MODE=lto,            CHECK=most|none] => "val3val5"
        let s5 = scenars.split(s3, "CHECK", &["some"]);
        let s6 = scenars.split(s3, "CHECK", &["most", "none"]);
        let s7 = scenars.split(s2, "CHECK", &["some"]);
        let s8 = scenars.split(s2, "CHECK", &["most", "none"]);

        let var1_ref = RawExpr::Name(QualifiedName {
            project: None,
            package: PackageName::None,
            name: SimpleName::Name("var1".to_string()),
        });
        let var2_ref = RawExpr::Name(QualifiedName {
            project: None,
            package: PackageName::None,
            name: SimpleName::Name("var2".to_string()),
        });
        let concat = var1_ref.ampersand(var2_ref);
        let concat_expr =
            ExprValue::new_with_raw(&concat, &gpr, &[], &mut scenars, s0, pkg)?;

        let mut expected = HashMap::new();
        expected.insert(s7, "val2val4".to_string());
        expected.insert(s8, "val2val5".to_string());
        expected.insert(s5, "val3val4".to_string());
        expected.insert(s6, "val3val5".to_string());
        assert_eq!(concat_expr, ExprValue::Str(expected));

        Ok(())
    }
}
