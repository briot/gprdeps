use crate::errors::Error;
use crate::gpr::GprFile;
use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
use crate::scenarios::{AllScenarios, Scenario};
use std::collections::HashMap;
use ustr::{Ustr, UstrMap};

/// Display the value of a variable on two columns:
///     <indent>scenario1      value1<eol>
///     <indent>scenar2        value2<eol>
fn two_columns<T>(
    map: &HashMap<Scenario, T>,
    scenarios: &AllScenarios,
    indent: &str,
    eol: &str,
    fmt: fn(&T) -> String,
) -> String {
    let mut col1 = Vec::new();
    for scenario in map.keys() {
        col1.push(scenarios.describe(*scenario));
    }
    let max = col1.iter().map(|s| s.len()).max().unwrap_or(0);
    let mut lines = map.iter()
        .enumerate()
        .map(|(idx, (_, val))| {
            format!("{}{:width$} {}", indent, col1[idx], fmt(val), width = max)
        })
        .collect::<Vec<_>>();
    lines.sort();
    lines.join(eol)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprValue {
    Str(HashMap<Scenario, Ustr>),
    StrList(HashMap<Scenario, Vec<Ustr>>),
    PathList(HashMap<Scenario, Vec<std::path::PathBuf>>),
}

impl ExprValue {
    /// An expression that always has the same static value for all scenarios
    pub fn new_with_str(s: Ustr) -> Self {
        ExprValue::new_with_str_and_scenario(s, Scenario::default())
    }

    pub fn new_with_str_and_scenario(s: Ustr, scenario: Scenario) -> Self {
        let mut m = HashMap::new();
        m.insert(scenario, s);
        ExprValue::Str(m)
    }

    // An expression value created as an empty list
    pub fn new_with_list(list: &[Ustr]) -> Self {
        let mut m = HashMap::new();
        m.insert(
            Scenario::default(),
            list.iter().map(|s| Ustr::from(s)).collect(),
        );

        ExprValue::StrList(m)
    }

    /// The expression is assumed to have a single value, for the default
    /// scenario (think of types).  Return that value.
    /// Otherwise panic
    pub fn as_list(&self) -> &Vec<Ustr> {
        match self {
            ExprValue::StrList(v) => &v[&Scenario::default()],
            _ => panic!("Expected a list {:?}", self),
        }
    }

    /// List all scenarios that have an impact on the variable's value
    //    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
    //        match self {
    //            ExprValue::Str(v) => useful.extend(v.keys()),
    //            ExprValue::StrList(v) => useful.extend(v.keys()),
    //            ExprValue::PathList(v) => useful.extend(v.keys()),
    //        }
    //    }

    /// Given a scenario variable, as setup via new_with_variable, prepares
    /// a mapping from one string value to the corresponding scenario.
    pub fn prepare_case_stmt(&self) -> Result<UstrMap<Scenario>, Error> {
        match self {
            ExprValue::Str(per_scenario) => {
                let mut result = UstrMap::default();
                for (s, v) in per_scenario {
                    result.insert(*v, *s);
                }
                Ok(result)
            }
            _ => Err(Error::VariableMustBeString),
        }
    }

    /// Evaluate a raw expression into its final value.
    /// The expression is initially seen in the context of one scenario (matching
    /// the case and when clauses), but its final value might be split into
    /// several scenarios if it is referencing another variable.
    pub fn new_with_raw(
        expr: &RawExpr,
        gpr: &GprFile, //  what project what this expression read in ?
        gpr_deps: &[&GprFile],
        scenars: &mut AllScenarios,
        scenar: Scenario,
        current_pkg: PackageName,
    ) -> Result<Self, Error> {
        match expr {
            RawExpr::Empty | RawExpr::Others => {
                panic!("{}: Cannot evaluate this expression {:?}", gpr, expr);
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
                        RawExpr::Str(v) => v,
                        _ => panic!(
                            "Expected static string for variable \
                                     name in {:?}",
                            expr
                        ),
                    };
                    let default = match args.get(1) {
                        None => ExprValue::new_with_str(Ustr::from("")),
                        Some(expr) => ExprValue::new_with_raw(
                            expr,
                            gpr,
                            gpr_deps,
                            scenars,
                            scenar,
                            current_pkg,
                        )?,
                    };
                    match &std::env::var(varname.as_str()) {
                        Ok(v) => Ok(ExprValue::new_with_str(Ustr::from(v))),
                        Err(_) => Ok(default),
                    }
                }
                _ => Err(Error::UnknownFunction(*n)),
            },
            RawExpr::FuncCall(_) => {
                Err(Error::UnknownFunction(Ustr::from(&format!("{:?}", expr))))
            }
            RawExpr::Name(q) => {
                Ok(gpr.lookup(q, gpr_deps, current_pkg)?.clone())
            }
            RawExpr::Str(s) => {
                Ok(ExprValue::new_with_str_and_scenario(*s, scenar))
            }
            RawExpr::List(ls) => {
                let mut m: HashMap<Scenario, Vec<Ustr>> = HashMap::new();
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
                                    v.push(v2);
                                    if let Some(s) =
                                        scenars.intersection(*s1, s2)
                                    {
                                        new_m.insert(s, v);
                                    }
                                }
                            }
                            m = new_m;
                        }
                        _ => Err(Error::ListCanOnlyContainStrings)?,
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
                                let mut res = v1.as_str().to_string();
                                res.push_str(v2.as_str());
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    m.insert(s, Ustr::from(&res));
                                }
                            }
                        }
                        Ok(ExprValue::Str(m))
                    }

                    (ExprValue::Str(_), _) => Err(Error::WrongAmpersand),

                    (ExprValue::StrList(ls), ExprValue::Str(rs)) => {
                        let mut m = HashMap::new();
                        for (s1, v1) in ls {
                            for (s2, v2) in &rs {
                                let mut res = v1.clone();
                                res.push(*v2);
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    m.insert(s, res);
                                }
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
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    m.insert(s, res);
                                }
                            }
                        }
                        Ok(ExprValue::StrList(m))
                    }

                    _ => Err(Error::WrongAmpersand),
                }
            }
        }
    }

    /// Merge two expression values.
    /// There must not be any conflicts (value set for the same scenario in
    /// both self and right, even if the values match).
    #[cfg(test)]
    fn merge_internal<T: Eq + std::fmt::Debug>(
        v_self: &mut HashMap<Scenario, T>,
        v_right: HashMap<Scenario, T>,
        scenars: &mut AllScenarios,
    ) -> Result<(), Error> {
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
                        Err(Error::CannotMerge)?;
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
    #[cfg(test)]
    pub fn merge(
        &mut self,
        right: ExprValue,
        scenars: &mut AllScenarios,
    ) -> Result<(), Error> {
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
            (s, r) => Err(Error::type_mismatch(s, r)),
        }
    }

    /// Display the expression.
    /// This is intended for debugging only.
    pub fn format(
        &self,
        scenarios: &AllScenarios,
        indent: &str,
        eol: &str,
    ) -> String {
        match self {
            ExprValue::Str(map) => {
                two_columns(map, scenarios, indent, eol, |s| {
                    format!("{}", s)
                })
            }
            ExprValue::StrList(map) => {
                two_columns(map, scenarios, indent, eol, |s| {
                    s.iter()
                        .map(|s| format!("{}", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            }
            ExprValue::PathList(map) => {
                two_columns(map, scenarios, indent, eol, |s| {
                    s.iter()
                        .map(|s| format!("{}", s.display()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::Error;
    use crate::gpr::GprFile;
    use crate::rawexpr::tests::{build_expr_list, build_expr_str};
    use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
    use crate::scenarios::tests::{split, try_add_variable};
    use crate::scenarios::{AllScenarios, Scenario};
    use crate::values::ExprValue;
    use std::collections::HashMap;
    use ustr::Ustr;

    #[test]
    fn test_eval() -> Result<(), Error> {
        let mut gpr = GprFile::new(std::path::Path::new("/"));
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
            ExprValue::new_with_str(Ustr::from("value")),
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
            ExprValue::new_with_str(Ustr::from("valuesuffix")),
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
            ExprValue::new_with_list(&[Ustr::from("val1"), Ustr::from("val2")])
        );

        // Evaluate a list of expressions
        let expr4 = RawExpr::List(vec![
            build_expr_str("value").ampersand(build_expr_str("suffix")),
            build_expr_str("val2"),
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
            ExprValue::new_with_list(&[
                Ustr::from("valuesuffix"),
                Ustr::from("val2")
            ]),
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
            ExprValue::new_with_list(&[
                Ustr::from("val1"),
                Ustr::from("val2"),
                Ustr::from("value")
            ]),
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
            ExprValue::new_with_list(&[
                Ustr::from("val1"),
                Ustr::from("val2"),
                Ustr::from("val3"),
                Ustr::from("val4")
            ]),
        );

        // Evaluate a qualified name

        gpr.declare(
            PackageName::None,
            SimpleName::Name(Ustr::from("var1")),
            ExprValue::new_with_str(Ustr::from("val1")),
        )?;

        let expr =
            build_expr_str("value").ampersand(RawExpr::Name(QualifiedName {
                project: None,
                package: PackageName::None,
                name: SimpleName::Name(Ustr::from("var1")),
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
            ExprValue::new_with_str(Ustr::from("valueval1")),
        );

        Ok(())
    }

    #[test]
    fn test_eval_scenario() -> Result<(), Error> {
        let mut gpr = GprFile::new(std::path::Path::new("/"));
        let mut scenars = AllScenarios::default();
        try_add_variable(&mut scenars, "MODE", &["debug", "optimize", "lto"])?;
        try_add_variable(&mut scenars, "CHECK", &["none", "some", "most"])?;
        let pkg = PackageName::None;
        let s0 = Scenario::default();
        let s2 =
            split(&mut scenars, s0, "MODE", &["debug", "optimize"]).unwrap();
        let s3 = split(&mut scenars, s0, "MODE", &["lto"]).unwrap();
        let s4 = split(&mut scenars, s0, "CHECK", &["some"]).unwrap();
        let s5 = split(&mut scenars, s0, "CHECK", &["most", "none"]).unwrap();

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
            SimpleName::Name(Ustr::from("var1")),
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
            SimpleName::Name(Ustr::from("var2")),
            var2,
        )?;

        // Computing the concatenation results in multiple possible values
        //   s2*s4=s7=[MODE=debug|optimize, CHECK=some]      => "val2val4"
        //   s2*s5=s8=[MODE=debug|optimize, CHECK=most|none] => "val2val5"
        //   s3*s4=s5=[MODE=lto,            CHECK=some]      => "val3val4"
        //   s3*s4=s6=[MODE=lto,            CHECK=most|none] => "val3val5"
        let s5 = split(&mut scenars, s3, "CHECK", &["some"]).unwrap();
        let s6 = split(&mut scenars, s3, "CHECK", &["most", "none"]).unwrap();
        let s7 = split(&mut scenars, s2, "CHECK", &["some"]).unwrap();
        let s8 = split(&mut scenars, s2, "CHECK", &["most", "none"]).unwrap();

        let var1_ref = RawExpr::Name(QualifiedName {
            project: None,
            package: PackageName::None,
            name: SimpleName::Name(Ustr::from("var1")),
        });
        let var2_ref = RawExpr::Name(QualifiedName {
            project: None,
            package: PackageName::None,
            name: SimpleName::Name(Ustr::from("var2")),
        });
        let concat = var1_ref.ampersand(var2_ref);
        let concat_expr =
            ExprValue::new_with_raw(&concat, &gpr, &[], &mut scenars, s0, pkg)?;

        let mut expected = HashMap::new();
        expected.insert(s7, Ustr::from("val2val4"));
        expected.insert(s8, Ustr::from("val2val5"));
        expected.insert(s5, Ustr::from("val3val4"));
        expected.insert(s6, Ustr::from("val3val5"));
        assert_eq!(concat_expr, ExprValue::Str(expected));

        Ok(())
    }

    /// A list expression is built with values that differ between scenarios.
    /// The resulting expression should therefore have different values for
    /// each scenario (four combinations here).
    #[test]
    fn list_in_scenar() -> Result<(), Error> {
        let raw = crate::gpr::tests::parse(
            r#"project P is
               type On_Off is ("on", "off");
               E1 : On_Off := external ("e1");
               E2 : On_Off := external ("e2");
               V := ("a", E1, E2, E1);
               end P;"#
        )?;
        let mut scenarios = crate::scenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
        gpr.print_details(&scenarios, true);
        crate::gpr::tests::assert_variable(
            &gpr,
            PackageName::None,
            "v",
            &scenarios,
            "e1=off,e2=off a, off, off, off\n\
             e1=off,e2=on  a, off, on, off\n\
             e1=on,e2=off  a, on, off, on\n\
             e1=on,e2=on   a, on, on, on",
        );
        Ok(())
    }

    /// A list expression is built via multiple case statements, and builds a
    /// value that depends on the scenario.  We must make sure that a "match"
    /// arm doesn't override the value of the expression in other scenarios.
    #[test]
    fn var_from_case() -> Result<(), Error> {
        let raw = crate::gpr::tests::parse(
            r#"project P is
               type On_Off is ("on", "off");
               E1 : On_Off := external ("e1");
               V := ("a");
               case E1 is
                  when "on"  => V := V & ("b");
                  when "off" => V := V & ("c");
               end case;

               --  Variable declared after first use of scenarios
               E2 : On_Off := external ("e2");
               case E2 is
                  when "on"  => V := V & ("d");
                  when "off" => V := V & ("e");
               end case;
            end P;
            "#,
        )?;
        let mut scenarios = crate::scenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
        gpr.print_details(&scenarios, true);
        crate::gpr::tests::assert_variable(
            &gpr,
            PackageName::None,
            "v",
            &scenarios,
            "foo",
        );

        Ok(())
    }
}
