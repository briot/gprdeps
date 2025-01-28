use crate::errors::Error;
use crate::gpr::GprFile;
use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
use crate::scenarios::{
    AllScenarios, Scenario, WhenClauseScenario, WhenContext,
};
use std::collections::HashMap;
use ustr::Ustr;

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
    let max = col1.iter().map(String::len).max().unwrap_or(0);
    let mut lines = map
        .iter()
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

    /// An expression that has a specific string value for one scenario, and no
    /// value for all others.
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

    /// Evaluate a raw expression into its final value.
    /// The expression is initially seen in the context of one scenario (matching
    /// the case and when clauses), but its final value might be split into
    /// several scenarios if it is referencing another variable.
    pub fn new_with_raw(
        expr: &RawExpr,
        gpr: &GprFile, //  what project what this expression read in ?
        gpr_deps: &[&GprFile],
        scenars: &mut AllScenarios,
        context: &WhenContext,
        current_pkg: PackageName,
    ) -> Result<Self, Error> {
        println!("MANU ExprValue.new_with_raw {:?}", expr);
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
                            context,
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
                Ok(ExprValue::new_with_str_and_scenario(*s, context.scenario))
            }
            RawExpr::List(ls) => {
                let mut m: HashMap<Scenario, Vec<Ustr>> = HashMap::new();
                let mut values = vec![];
                for expr in ls {
                    // Each element of the list is an expression, which could
                    // have a different value for each scenario.
                    let s = ExprValue::new_with_raw(
                        expr,
                        gpr,
                        gpr_deps,
                        scenars,
                        context,
                        current_pkg,
                    )?;
                    println!("MANU element in list {:?}", s);
                    match s {
                        ExprValue::Str(per_scenario) => {
                            //                            for (s, v) in per_scenario {
                            //                                ExprValue::split_hash(
                            //                                    &mut m,
                            //                                    when,
                            //                                    &None,  // all scenarios
                            //                                    scenars,
                            //                                );
                            //                            }

                            // The string is always defined for the current
                            // scenario.
                            assert_eq!(per_scenario.len(), 1);
                            assert_eq!(
                                *per_scenario.keys().next().unwrap(),
                                context.scenario,
                            );

                            // The string's scenario doesn't change anything in
                            // the list, so we can just add it.
                            values.push(*per_scenario.values().next().unwrap());
                        }
                        _ => Err(Error::ListCanOnlyContainStrings)?,
                    }
                }
                m.insert(context.scenario, values);
                Ok(ExprValue::StrList(m))
            }
            RawExpr::Ampersand((left, right)) => {
                let mut l_eval = ExprValue::new_with_raw(
                    left,
                    gpr,
                    gpr_deps,
                    scenars,
                    context,
                    current_pkg,
                )?;
                let mut r_eval = ExprValue::new_with_raw(
                    right,
                    gpr,
                    gpr_deps,
                    scenars,
                    context,
                    current_pkg,
                )?;
                println!("MANU RawExpr {:?} & {:?}", l_eval, r_eval);

                let mut l_active = None;
                let mut r_active = None;
                for c in &context.clauses {
                    l_active =
                        Some(l_eval.split_in_place(c, &l_active, scenars));
                    r_active =
                        Some(r_eval.split_in_place(c, &r_active, scenars));
                }
                println!(
                    "MANU after splitting {:?}/{:?} & {:?}/{:?}",
                    l_eval, l_active, r_eval, r_active
                );

                match (l_eval, r_eval) {
                    (ExprValue::Str(ls), ExprValue::Str(rs)) => {
                        let mut m = HashMap::new();
                        for (s1, v1) in ls {
                            for (s2, v2) in &rs {
                                // The string v1&v2 is only meaningful for the
                                // mode that is the intersection of s1 and s2.
                                // In other modes, v1 or v2 are considered the
                                // empty string.
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    let mut res = v1.as_str().to_string();
                                    res.push_str(v2.as_str());
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
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    let mut res = v1.clone();
                                    res.push(*v2);
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
                                if let Some(s) = scenars.intersection(s1, *s2) {
                                    let mut res = v1.clone();
                                    res.extend(v2.clone());
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

    /// Computes the value of an expression, applying case statements.
    /// For instance, if we start with the following variables:
    ///     s1: Scenario = {"E1": "on"}
    ///     s2: Scenario = {"E1": "off"}
    ///     s3: Scenario = {"E2": "on" }
    ///     s4: Scenario = {"E2": "off" }
    ///     V1 = { s1: "a", s2+s3: "b", s2+s4: "c" }
    ///     V2 = { s1: "d", s2: "e" }
    ///
    /// and we see the following code in the project file:
    ///    case E1 is
    ///       when "on" =>           --  scenario s1
    ///           case E2 is
    ///               when "on" =>   --  scenario s3
    ///                   V3 := V1 & V2;
    ///
    /// Then the following occurs:
    ///  * We will need V1, so we split its value so that we have individual
    ///    entries for scenario s1.  This split must not change the overall
    ///    value of V1.   In practice, V1 is unchanged, but we not the "active"
    ///    entries (upper-cased here)
    ///     V1 = { S1: "a", s2+s3: "b", s2+s4: "c" }
    ///  * We then splitV1 for scenario s3 (E2=on).  Only active entries are
    ///    changed.  Again we mark active entries
    ///     V1 = { S1+S3: "a", s1+s4: "a", s2+s3: "b", s2+s4: "c" }
    ///  * Same two splits for V2
    ///     V2 = { S1: "d", s2: "e"}
    ///     V2 = { S1+S3: "d", s1+s4: "d", s2: "e"}
    ///  * We then combine the active entries from both variables.  This is a
    ///    cross product
    ///     V3 = { s1+s3: "ad" }
    ///  * Note that if instead of setting V3 we were setting V1, we need to
    ///    preserve the existing non-active entries, so we would end with:
    ///     V1 = { s1+s3: "ad", s1+s4: "a", s2+s3: "b", s2+s4: "c" }
    pub fn split_hash<T: Clone>(
        map: &mut HashMap<Scenario, T>,
        when: &WhenClauseScenario,
        active: &Option<Vec<Scenario>>, // Only modify those entries if specified
        scenars: &mut AllScenarios,
    ) -> Vec<Scenario> {
        let mut res = HashMap::new();
        let mut new_active = Vec::new();
        for (scenario, v) in map.iter_mut() {
            if active.as_ref().map_or(true, |l| l.contains(scenario)) {
                if let Some(s) = scenars.intersection(*scenario, when.scenario)
                {
                    new_active.push(s);
                    res.insert(s, v.clone());
                }
                if let Some(n) = when.negate_scenario {
                    if let Some(s) = scenars.intersection(*scenario, n) {
                        res.insert(s, v.clone());
                    }
                }
            } else {
                res.insert(*scenario, v.clone());
            }
        }
        *map = res;
        new_active
    }

    pub fn split_in_place(
        &mut self,
        when: &WhenClauseScenario,
        active: &Option<Vec<Scenario>>, // Only modify those entries if specified
        scenars: &mut AllScenarios,
    ) -> Vec<Scenario> {
        match self {
            ExprValue::Str(map) => {
                ExprValue::split_hash(map, when, active, scenars)
            }
            ExprValue::StrList(map) => {
                ExprValue::split_hash(map, when, active, scenars)
            }
            ExprValue::PathList(map) => {
                ExprValue::split_hash(map, when, active, scenars)
            }
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
                two_columns(map, scenarios, indent, eol, |s| format!("{}", s))
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
    use crate::scenarios::{AllScenarios, WhenContext};
    use crate::values::ExprValue;
    use ustr::Ustr;

    #[test]
    fn test_eval() -> Result<(), Error> {
        let mut gpr = GprFile::new(std::path::Path::new("/"));
        let mut scenars = AllScenarios::default();
        let context = WhenContext::new();
        let pkg = PackageName::None;

        // Evaluate a string
        let expr1 = build_expr_str("value");
        assert_eq!(
            ExprValue::new_with_raw(
                &expr1,
                &gpr,
                &[],
                &mut scenars,
                &context,
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
                &context,
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
                &context,
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
                &context,
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
                &context,
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
                &context,
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
            &context,
            &mut scenars,
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
                &context,
                pkg
            )?,
            ExprValue::new_with_str(Ustr::from("valueval1")),
        );

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
               end P;"#,
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
               E1 : On_Off := external ("e1");  --  in practice, undefined
               V := ("a");
               case E1 is
                  when "on" | "off" => V := V & "f";
                  when others => null;
               end case;
               case E1 is
                  when "on"  => V := V & "b";
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
            "e1=off,e2=off a, f, c, e\n\
             e1=off,e2=on  a, f, c, d\n\
             e1=on,e2=off  a, f, b, e\n\
             e1=on,e2=on   a, f, b, d",
        );

        Ok(())
    }
}
