use crate::errors::Error;
use crate::gpr::GprFile;
use crate::packagename::PackageName;
use crate::perscenario::PerScenario;
use crate::qualifiedname::QualifiedName;
use crate::rawexpr::RawExpr;
use crate::scenarios::{AllScenarios, Scenario, WhenContext};
use crate::simplename::SimpleName;
use ustr::Ustr;

#[derive(Clone, Debug, PartialEq)]
pub enum ExprValue {
    Str(PerScenario<Ustr>),
    StrList(PerScenario<Vec<Ustr>>),
    PathList(PerScenario<Vec<std::path::PathBuf>>),
}

impl ExprValue {
    /// An expression that always has the same static value for all scenarios
    pub fn new_with_str(s: Ustr) -> Self {
        ExprValue::new_with_str_and_scenario(s, Scenario::default())
    }

    /// An expression that has a specific string value for one scenario, and no
    /// value for all others.
    pub fn new_with_str_and_scenario(s: Ustr, scenario: Scenario) -> Self {
        ExprValue::Str(PerScenario::new(s, scenario))
    }

    // An expression value created as an empty list
    pub fn new_with_list(list: &[Ustr]) -> Self {
        ExprValue::StrList(PerScenario::new(list.to_vec(), Scenario::default()))
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
                            //  for (s, v) in per_scenario {
                            //      split_hash(
                            //          &mut m,
                            //          when,
                            //          &None,  // all scenarios
                            //          scenars,
                            //      );
                            //  }

                            // The string is always defined for the current
                            // scenario.
                            assert_eq!(per_scenario.values.len(), 1);
                            assert_eq!(
                                *per_scenario.values.keys().next().unwrap(),
                                context.scenario,
                            );

                            // The string's scenario doesn't change anything in
                            // the list, so we can just add it.
                            values.push(
                                *per_scenario.values.values().next().unwrap(),
                            );
                        }
                        _ => Err(Error::ListCanOnlyContainStrings)?,
                    }
                }
                Ok(ExprValue::StrList(PerScenario::new(
                    values,
                    context.scenario,
                )))
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
                match (&mut l_eval, &mut r_eval) {
                    (ExprValue::Str(ls), ExprValue::Str(rs)) => {
                        Ok(ExprValue::Str(ls.merge(
                            rs,
                            context,
                            scenars,
                            |v1, v2| {
                                let mut res = v1.as_str().to_string();
                                res.push_str(v2.as_str());
                                Ustr::from(&res)
                            },
                        )))
                    }
                    (ExprValue::StrList(ls), ExprValue::Str(rs)) => {
                        Ok(ExprValue::StrList(ls.merge(
                            rs,
                            context,
                            scenars,
                            |v1, v2| {
                                let mut res = v1.clone();
                                res.push(*v2);
                                res
                            },
                        )))
                    }
                    (ExprValue::StrList(ls), ExprValue::StrList(rs)) => {
                        Ok(ExprValue::StrList(ls.merge(
                            rs,
                            context,
                            scenars,
                            |v1, v2| {
                                let mut res = v1.clone();
                                res.extend(v2.clone());
                                res
                            },
                        )))
                    }
                    _ => Err(Error::WrongAmpersand),
                }
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
                map.two_columns(scenarios, indent, eol, |s| format!("{}", s))
            }
            ExprValue::StrList(map) => {
                map.two_columns(scenarios, indent, eol, |s| {
                    s.iter()
                        .map(|s| format!("{}", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            }
            ExprValue::PathList(map) => {
                map.two_columns(scenarios, indent, eol, |s| {
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
    use crate::packagename::PackageName;
    use crate::qualifiedname::QualifiedName;
    use crate::rawexpr::tests::{build_expr_list, build_expr_str};
    use crate::rawexpr::RawExpr;
    use crate::scenarios::{AllScenarios, WhenContext};
    use crate::simplename::SimpleName;
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
