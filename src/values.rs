use crate::{
    allscenarios::AllScenarios, errors::Error, gpr::GprFile,
    packagename::PackageName, perscenario::PerScenario,
    qualifiedname::QualifiedName, rawexpr::RawExpr, scenarios::Scenario,
    simplename::SimpleName,
};
use itertools::join;
use std::collections::HashSet;
use ustr::Ustr;

#[derive(Clone, Debug, PartialEq)]
pub enum ExprValue {
    Str(PerScenario<Ustr>),
    StrList(PerScenario<Vec<Ustr>>),
}

impl ExprValue {
    /// An expression that always has the same static value for all scenarios
    pub fn new_with_str(s: Ustr) -> Self {
        ExprValue::Str(PerScenario::new(s))
    }

    // An expression value created as a list of strings
    pub fn new_with_list(list: Vec<Ustr>) -> Self {
        ExprValue::StrList(PerScenario::new(list))
    }

    /// Evaluate a raw expression into its final value.
    /// The expression is initially seen in the context of one scenario (matching
    /// the case and when clauses), but its final value might be split into
    /// several scenarios if it is referencing another variable.
    pub fn new_with_raw(
        expr: &RawExpr,
        gpr: &GprFile, //  what project was this expression read in ?
        gpr_deps: &[&GprFile],
        scenars: &mut AllScenarios,
        context: Scenario,
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
                let mut v = PerScenario::new(Ustr::default());
                let s2 = PerScenario::new(*s); // static value, all scenarios
                v.update(&s2, context, scenars, |v1, v2| *v1 = *v2);
                Ok(ExprValue::Str(v))
            }
            RawExpr::List(ls) => {
                let mut values = PerScenario::new(Vec::new());

                for expr in ls {
                    // Each element of the list is an expression, which could
                    // have a different value for each scenario.
                    let mut s = ExprValue::new_with_raw(
                        expr,
                        gpr,
                        gpr_deps,
                        scenars,
                        context,
                        current_pkg,
                    )?;
                    match &mut s {
                        ExprValue::Str(per_scenario) => {
                            // The string's scenario doesn't change anything in
                            // the list, so we can just add it.
                            values.update(
                                per_scenario,
                                context,
                                scenars,
                                |v1, v2| v1.push(*v2),
                            );
                        }
                        _ => Err(Error::ListCanOnlyContainStrings)?,
                    }
                }
                Ok(ExprValue::StrList(values))
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
                match (&mut l_eval, &mut r_eval) {
                    (ExprValue::Str(ls), ExprValue::Str(rs)) => {
                        ls.update(rs, context, scenars, |v1, v2| {
                            let mut res = v1.as_str().to_string();
                            res.push_str(v2.as_str());
                            *v1 = Ustr::from(&res);
                        });
                    }
                    (ExprValue::StrList(ls), ExprValue::Str(rs)) => {
                        ls.update(rs, context, scenars, |v1, v2| v1.push(*v2));
                    }
                    (ExprValue::StrList(ls), ExprValue::StrList(rs)) => {
                        ls.update(rs, context, scenars, |v1, v2| v1.extend(v2));
                    }
                    _ => Err(Error::WrongAmpersand)?,
                }
                Ok(l_eval)
            }
        }
    }

    /// Find all scenarios that result in different values in the project
    pub fn find_used_scenarios(&self, scenars: &mut HashSet<Scenario>) {
        match self {
            ExprValue::Str(a) => a.find_used_scenarios(scenars),
            ExprValue::StrList(a) => a.find_used_scenarios(scenars),
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
                map.two_columns(scenarios, indent, eol, |s| s.to_string())
            }
            ExprValue::StrList(map) => {
                map.two_columns(scenarios, indent, eol, |s| {
                    join(s.iter(), ", ")
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        allscenarios::AllScenarios,
        errors::Error,
        gpr::GprFile,
        graph::NodeIndex,
        packagename::PackageName,
        qualifiedname::QualifiedName,
        rawexpr::tests::{build_expr_list, build_expr_str},
        rawexpr::RawExpr,
        scenarios::Scenario,
        simplename::SimpleName,
        values::ExprValue,
    };
    use ustr::Ustr;

    macro_rules! assert_err {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
            }
        }
    }

    #[test]
    fn test_eval() -> Result<(), Error> {
        let mut gpr = GprFile::new(
            std::path::Path::new("/"),
            false,
            false,
            false,
            NodeIndex::new(0),
        );
        let mut scenars = AllScenarios::default();
        let pkg = PackageName::None;

        // Evaluate a string
        let expr1 = build_expr_str("value");
        assert_eq!(
            ExprValue::new_with_raw(
                &expr1,
                &gpr,
                &[],
                &mut scenars,
                Scenario::default(),
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
                Scenario::default(),
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
                Scenario::default(),
                pkg
            )?,
            ExprValue::new_with_list(vec![
                Ustr::from("val1"),
                Ustr::from("val2")
            ])
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
                Scenario::default(),
                pkg
            )?,
            // " valuesuffix, val2",
            ExprValue::new_with_list(vec![
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
                Scenario::default(),
                pkg
            )?,
            ExprValue::new_with_list(vec![
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
                Scenario::default(),
                pkg
            )?,
            ExprValue::new_with_list(vec![
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
            Scenario::default(),
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
                Scenario::default(),
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
        let mut scenarios = crate::allscenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
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
        let mut scenarios = crate::allscenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
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

    #[test]
    fn split_full() -> Result<(), Error> {
        let raw = crate::gpr::tests::parse(
            r#"project P is
               type On_Off is ("on", "off");
               E1 : On_Off := external ("e1");
               E2 : On_Off := external ("e2");

               V := "a";
               case E1 is
                  when "on" =>
                     case E2 is
                        when "on" =>
                           V := "b";
                      end case;
               end case;
            end P;
            "#,
        )?;
        let mut scenarios = crate::allscenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
        crate::gpr::tests::assert_variable(
            &gpr,
            PackageName::None,
            "v",
            &scenarios,
            "e1=off      a\n\
             e1=on,e2=on b\n\
             e2=off      a",
        );
        Ok(())
    }

    /// Check what happens when we have too many scenario variables and too
    /// many valid values (overflow of the Mask)
    #[test]
    fn mask_overflow() -> Result<(), Error> {
        let raw = crate::gpr::tests::parse(
            r#"project P is
               type T is ("a", "b", "c", "d");
               E1 : T := external ("e1");
               E2 : T := external ("e2");
               E3 : T := external ("e3");
               E4 : T := external ("e4");
               E5 : T := external ("e5");
               E6 : T := external ("e6");
               E7 : T := external ("e7");
               E8 : T := external ("e8");
               E9 : T := external ("e9");
               E10 : T := external ("e10");
               E11 : T := external ("e11");
               E12 : T := external ("e12");
               E13 : T := external ("e13");
               E14 : T := external ("e14");
               E15 : T := external ("e15");
               E16 : T := external ("e16");
               E17 : T := external ("e17");
               end P;
            "#,
        )?;
        let mut scenarios = crate::allscenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios);
        assert_err!(gpr, Err(Error::WithPath {error, ..})
            if matches!(*error, Error::TooManyScenarioVariables));

        Ok(())
    }
}
