use crate::gpr::GPR;
use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
use crate::scenarios::{AllScenarios, Scenario};
use std::collections::{HashMap, HashSet};

/// The value of a variable or attribute, evaluated in one scenario
#[derive(Clone, Debug, PartialEq)]
pub enum OneScenario {
    StaticString(String),
    List(Vec<String>),
}

/// The value of a variable or attribute, in all scenarios
#[derive(Clone, Debug, PartialEq)]
pub struct ExprValue(HashMap<Scenario, OneScenario>);

impl ExprValue {
    /// An expression that always has the same static value for all scenarios
    pub fn new_static_str(s: &str) -> Self {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            OneScenario::StaticString(s.to_string()),
        );
        ExprValue(m)
    }

    // An expression value created as an empty list
    pub fn from_list(list: &[&str]) -> Self {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            OneScenario::List(list.iter().map(|s| s.to_string()).collect())
        );
        ExprValue(m)
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
    pub fn from_scenario_variable(
        scenarios: &mut AllScenarios,
        varname: &str,
        type_values: &ExprValue,
    ) -> Self {
        let valid = type_values.as_static_list();
        let mut m = HashMap::new();
        let s0 = Scenario::default();
        for v in valid {
            let s1 = scenarios.split(s0, varname, &[v]);
            m.insert(s1, OneScenario::StaticString(v.clone()));
        }
        ExprValue(m)
    }

    /// Assumes the expression is a static string valid for all scenarios and
    /// return it.
    pub fn as_static_str(&self) -> &String {
        if self.0.len() != 1 {
            panic!("Expected no variants {:?}", self.0);
        }
        match &self.0[&Scenario::default()] {
            OneScenario::StaticString(s) => s,
            OneScenario::List(_) => panic!("Expected a string {:?}", self.0),
        }
    }

    /// The expression is assumed to have a single value, for the default
    /// scenario (think of types).  Return that value.
    /// Otherwise panic
    pub fn as_static_list(&self) -> &Vec<String> {
        if self.0.len() != 1 {
            panic!("Expected no variants {:?}", self.0);
        }
        match &self.0[&Scenario::default()] {
            OneScenario::StaticString(_) => {
                panic!("Expected a list {:?}", self.0)
            }
            OneScenario::List(v) => v,
        }
    }

    /// Find which scenarios are actually useful for this project
    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
        for s in self.0.keys() {
            useful.insert(*s);
        }
    }

    /// Return all strings used in any scenario
    pub fn find_all_str(&self) -> HashSet<&String> {
        let mut result = HashSet::new();
        for v in self.0.values() {
            match v {
                OneScenario::List(v) => {
                    for s in v {
                        result.insert(s);
                    }
                }
                OneScenario::StaticString(s) => {
                    result.insert(s);
                }
            }
        }
        result
    }

    /// Return a mapping from one string value to the corresponding scenario.
    /// This is used for scenario variables used in a case statement.
    pub fn prepare_case_stmt(
        &self,
    ) -> Result<HashMap<String, Scenario>, String> {
        let mut result = HashMap::new();
        for (s, v) in &self.0 {
            match v {
                OneScenario::List(_) => {
                    Err(format!(
                        "List cannot be used in a case statement: {:?}",
                        self.0
                    ))?;
                }
                OneScenario::StaticString(v) => {
                    result.insert(v.clone(), *s);
                }
            }
        }
        Ok(result)
    }

    /// Evaluate a raw expression into its final value.
    /// The expression is initially seen in the context of one scenario (matching
    /// the case and when clauses), but its final value might be split into
    /// several scenarios if it is referencing another variable.
    pub fn eval(
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
                        None => ExprValue::new_static_str(""),
                        Some(expr) => ExprValue::eval(
                            expr,
                            gpr,
                            gpr_deps,
                            scenars,
                            scenar,
                            current_pkg,
                        )?,
                    };
                    match &std::env::var(varname) {
                        Ok(v) => Ok(ExprValue::new_static_str(v)),
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
                let mut m = HashMap::new();
                m.insert(scenar, OneScenario::StaticString(s.clone()));
                Ok(ExprValue(m))
            }
            RawExpr::List(ls) => {
                let mut m = HashMap::new();
                m.insert(Scenario::default(), OneScenario::List(vec![]));

                for expr in ls {
                    let s = ExprValue::eval(
                        expr,
                        gpr,
                        gpr_deps,
                        scenars,
                        scenar,
                        current_pkg,
                    )?;
                    let mut new_m = HashMap::new();
                    for (s2, v2) in s.0 {
                        match v2 {
                            OneScenario::StaticString(st) => {
                                for (s1, v1) in &m {
                                    if let OneScenario::List(ls) = v1 {
                                        let mut v = ls.clone();
                                        v.push(st.clone());
                                        new_m.insert(
                                            scenars.intersection(*s1, s2)?,
                                            OneScenario::List(v),
                                        );
                                    }
                                }
                            }
                            OneScenario::List(_) => Err(format!(
                                "{}: lists can only contain strings",
                                gpr
                            ))?,
                        }
                    }
                    m = new_m;
                }
                Ok(ExprValue(m))
            }
            RawExpr::Ampersand((left, right)) => {
                let mut m = HashMap::new();
                let l_eval = ExprValue::eval(
                    left,
                    gpr,
                    gpr_deps,
                    scenars,
                    scenar,
                    current_pkg,
                )?;
                let r_eval = ExprValue::eval(
                    right,
                    gpr,
                    gpr_deps,
                    scenars,
                    scenar,
                    current_pkg,
                )?;

                for (s1, v1) in l_eval.0 {
                    match v1 {
                        OneScenario::StaticString(ls) => {
                            for (s2, v2) in &r_eval.0 {
                                match v2 {
                                    OneScenario::StaticString(rs) => {
                                        // The string v1&v2 is only meaningful
                                        // for the mode that is the intersection
                                        // of s1 and s2.  In other modes, v1 or
                                        // v2 are considered the empty string.
                                        let mut res = ls.clone();
                                        res.push_str(rs);
                                        m.insert(
                                            scenars.intersection(s1, *s2)?,
                                            OneScenario::StaticString(res),
                                        );
                                    },
                                    OneScenario::List(_) =>
                                        Err(format!(
                                            "{}: cannot concatenate string and list",
                                            gpr))?,
                                }
                            }
                        }
                        OneScenario::List(ls) => {
                            for (s2, v2) in &r_eval.0 {
                                match v2 {
                                    OneScenario::StaticString(rs) => {
                                        let mut res = ls.clone();
                                        res.push(rs.clone());
                                        m.insert(
                                            scenars.intersection(s1, *s2)?,
                                            OneScenario::List(res),
                                        );
                                    }
                                    OneScenario::List(rs) => {
                                        let mut res = ls.clone();
                                        for s in rs {
                                            res.push(s.clone());
                                        }
                                        m.insert(
                                            scenars.intersection(s1, *s2)?,
                                            OneScenario::List(res),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(ExprValue(m))
            }
        }
    }

    /// Find whether (scenar, value) can be merged with any existing state
    /// in self.
    fn find_mergeable(
        &self,
        scenar: Scenario,
        value: &OneScenario,
        scenars: &mut AllScenarios,
    ) -> Option<(Scenario, Scenario)> {
        for (s, v) in &self.0 {
            if v == value {
                if let Some(s2) = scenars.union(*s, scenar) {
                    return Some((s2, *s));
                }
            }
        }
        None
    }

    /// Merge two expression values.
    /// There must not be any conflicts (value set for the same scenario in
    /// both self and right, even if the values match).
    pub fn merge(
        &mut self,
        right: ExprValue,
        scenars: &mut AllScenarios,
    ) -> Result<(), String> {
        for (s, v) in &right.0 {
            match self.find_mergeable(*s, v, scenars) {
                None => {
                    if self.0.contains_key(s) {
                        Err(format!(
                            "Cannot merge two values, the same scenario occurs \
                             twice {}",
                            s))?;
                    }
                    self.0.insert(*s, v.clone());
                }
                Some((new_s, old_s)) => {
                    let old = self.0.remove(&old_s);
                    self.0.insert(new_s, old.unwrap());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::gpr::GPR;
    use crate::graph::NodeIndex;
    use crate::rawexpr::tests::{build_expr_list, build_expr_str};
    use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
    use crate::scenarios::{AllScenarios, Scenario};
    use crate::values::{ExprValue, OneScenario};
    use std::collections::HashMap;

    fn build_value_str(s: &str) -> ExprValue {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            OneScenario::StaticString(s.to_string()),
        );
        ExprValue(m)
    }

    fn build_value_list(s: &[&str]) -> ExprValue {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            OneScenario::List(s.iter().map(|st| st.to_string()).collect()),
        );
        ExprValue(m)
    }

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
            ExprValue::eval(&expr1, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_str("value"),
        );

        // Concatenate two strings
        let expr2 = build_expr_str("value").ampersand(build_expr_str("suffix"));
        assert_eq!(
            ExprValue::eval(&expr2, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_str("valuesuffix"),
        );

        // Evaluate a list of strings
        let expr3 = build_expr_list(&["val1", "val2"]);
        assert_eq!(
            ExprValue::eval(&expr3, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_list(&["val1", "val2"])
        );

        // Evaluate a list of expressions
        let expr4 = RawExpr::List(vec![
            Box::new(
                build_expr_str("value").ampersand(build_expr_str("suffix")),
            ),
            Box::new(build_expr_str("val2")),
        ]);
        assert_eq!(
            ExprValue::eval(&expr4, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_list(&["valuesuffix", "val2"]),
        );

        // Concatenate list and string
        let expr4 = build_expr_list(&["val1", "val2"])
            .ampersand(build_expr_str("value"));
        assert_eq!(
            ExprValue::eval(&expr4, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_list(&["val1", "val2", "value"]),
        );

        // Concatenate two lists
        let expr5 = build_expr_list(&["val1", "val2"])
            .ampersand(build_expr_list(&["val3", "val4"]));
        assert_eq!(
            ExprValue::eval(&expr5, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_list(&["val1", "val2", "val3", "val4"]),
        );

        // Evaluate a qualified name

        gpr.declare(
            PackageName::None,
            SimpleName::Name("var1".to_string()),
            build_value_str("val1"),
        )?;

        let expr =
            build_expr_str("value").ampersand(RawExpr::Name(QualifiedName {
                project: None,
                package: PackageName::None,
                name: SimpleName::Name("var1".to_string()),
            }));
        assert_eq!(
            ExprValue::eval(&expr, &gpr, &[], &mut scenars, scenar, pkg)?,
            build_value_str("valueval1"),
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
        let mut var1 = ExprValue::eval(
            &build_expr_str("val2"),
            &gpr,
            &[],
            &mut scenars,
            s2,
            pkg,
        )?;
        var1.merge(
            ExprValue::eval(
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
        let mut var2 = ExprValue::eval(
            &build_expr_str("val4"),
            &gpr,
            &[],
            &mut scenars,
            s4,
            pkg,
        )?;
        var2.merge(
            ExprValue::eval(
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
            ExprValue::eval(&concat, &gpr, &[], &mut scenars, s0, pkg)?;

        let mut expected = HashMap::new();
        expected.insert(s7, OneScenario::StaticString("val2val4".to_string()));
        expected.insert(s8, OneScenario::StaticString("val2val5".to_string()));
        expected.insert(s5, OneScenario::StaticString("val3val4".to_string()));
        expected.insert(s6, OneScenario::StaticString("val3val5".to_string()));
        assert_eq!(concat_expr, ExprValue(expected));

        Ok(())
    }
}
