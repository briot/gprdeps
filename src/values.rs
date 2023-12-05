use crate::errors::Error;
use crate::gpr::GprFile;
use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
use crate::scenarios::{AllScenarios, Scenario, EMPTY_SCENARIO};
use std::collections::HashMap;
use std::collections::HashSet;
use ustr::{Ustr, UstrMap, UstrSet};


#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExprValue<T>(HashMap<Scenario, T>);

struct CrossJoinIter<'a, T, U> {
    left_iter: std::collections::hash_map::Iter<'a, Scenario, T>,
    left_val: Option<(&'a Scenario, &'a T)>,
    right_iter: Option<std::collections::hash_map::Iter<'a, Scenario, U>>,
    right: &'a ExprValue<U>,
    scenarios: &'a mut AllScenarios,
}

impl<'a, T, U> CrossJoinIter<'a, T, U> {
    fn new(
        left_iter: std::collections::hash_map::Iter<'a, Scenario, T>,
        right: &'a ExprValue<U>,
        scenarios: &'a mut AllScenarios,
    ) -> Self {
        CrossJoinIter {
            left_iter,
            left_val: None,
            right,
            right_iter: None,
            scenarios,
        }
    }
}

impl<'a, T, U> Iterator for CrossJoinIter<'a, T, U> {
    type Item = (Scenario, (&'a T, &'a U));

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.left_val.is_none() {
                match self.left_iter.next() {
                    None => return None,
                    Some(v) => {
                        self.left_val = Some(v);
                    }
                }
            }
            match (self.left_val, self.right_iter.as_mut().and_then(|r| r.next())) {
                (None, _) => return None,
                (Some(_), None) => {
                    self.right_iter = Some(self.right.0.iter());
                },
                (Some((left_s, left_v)), Some((right_s, right_v))) => {
                    let s = self.scenarios.intersection(*left_s, *right_s);
                    if s != EMPTY_SCENARIO {
                        return Some((s, (left_v, right_v)));
                    }
                }
            }
        }
    }
}

impl ExprValue<Ustr> {

    /// An expression that always has the same static value for all scenarios
    pub fn new_with_str(s: Ustr) -> Self {
        ExprValue::new_with_str_and_scenario(s, Scenario::default())
    }

    pub fn new_with_str_and_scenario(s: Ustr, scenario: Scenario) -> Self {
        let mut m = ExprValue(HashMap::new());
        m.0.insert(scenario, s);
        m
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
        varname: Ustr,
        type_values: &ExprValue<Ustr>,
    ) -> Self {
        let mut m = ExprValue(HashMap::new());
        let s0 = Scenario::default();
        for v in &type_values {
            let mut onevalue = UstrSet::default();
            onevalue.insert(*v);
            let s1 = scenarios.split(s0, varname, onevalue);
            m.0.insert(s1, *v);
        }
        m
    }

}

impl ExprValue<Vec<Ustr>> {

    // An expression value created as an empty list
    pub fn new_with_list(list: &[Ustr]) -> Self {
        let mut m = ExprValue(HashMap::new());
        m.0.insert(
            Scenario::default(),
            list.iter().map(|s| Ustr::from(s)).collect(),
        );
        m
    }

    /// The expression is assumed to have a single value, for the default
    /// scenario (think of types).  Return that value.
    /// Otherwise panic
    pub fn as_list(&self) -> &Vec<Ustr> {
        &self.0[&Scenario::default]
    }
}


impl<T> ExprValue<T> {

    pub fn crossjoin<'a, U>(
        &'a self,
        right: &'a ExprValue<U>,
        scenarios: &'a mut AllScenarios,
    ) -> CrossJoinIter<'a, T, U> {
        CrossJoinIter::new(self.0.iter(), right, scenarios)
    }

    /// List all scenarios that have an impact on the variable's value
    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
        match self {
            ExprValue::Str(v) => useful.extend(v.0.keys()),
            ExprValue::StrList(v) => useful.extend(v.0.keys()),
            ExprValue::PathList(v) => useful.extend(v.0.keys()),
        }
    }

    /// Given a scenario variable, as setup via new_with_variable, prepares
    /// a mapping from one string value to the corresponding scenario.
    pub fn prepare_case_stmt(&self) -> Result<UstrMap<Scenario>, Error> {
        match self {
            ExprValue::Str(per_scenario) => {
                let mut result = UstrMap::default();
                for (s, v) in &per_scenario.0 {
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
                let mut m: ExprValue<Vec<Ustr>> = ExprValue(HashMap::new());
                m.0.insert(scenar, vec![]);
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

                            let mut new_m = ExprValue(HashMap::new());
                            for (s2, v2) in per_scenario.0 {
                                for (s1, v1) in &m.0 {
                                    let mut v = v1.clone();
                                    v.push(v2);
                                    new_m.0.insert(
                                        scenars.intersection(*s1, s2),
                                        v,
                                    );
                                }
                            }
                            m = new_m;
                        }
                        _ => Err(Error::ListCanOnlyContainStrings)?,
                    }
                }
                Ok(m)
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
                        let mut m = ExprValue(HashMap::new());
                        for (s1, v1) in ls.0 {
                            for (s2, v2) in &rs.0 {
                                // The string v1&v2 is only meaningful for the
                                // mode that is the intersection of s1 and s2.
                                // In other modes, v1 or v2 are considered the
                                // empty string.
                                let mut res = v1.as_str().to_string();
                                res.push_str(v2.as_str());
                                m.0.insert(
                                    scenars.intersection(s1, *s2),
                                    Ustr::from(&res),
                                );
                            }
                        }
                        Ok(m)
                    }

                    (ExprValue::Str(_), _) => Err(Error::WrongAmpersand),

                    (ExprValue::StrList(ls), ExprValue::Str(rs)) => {
                        let mut m = ExprValue(HashMap::new());
                        for (s1, v1) in ls.0 {
                            for (s2, v2) in &rs.0 {
                                let mut res = v1.clone();
                                res.push(*v2);
                                m.0.insert(scenars.intersection(s1, *s2), res);
                            }
                        }
                        Ok(ExprValue::StrList(m))
                    }

                    (ExprValue::StrList(ls), ExprValue::StrList(rs)) => {
                        let mut m = ExprValue(HashMap::new());
                        for (s1, v1) in ls.0 {
                            for (s2, v2) in &rs.0 {
                                let mut res = v1.clone();
                                res.extend(v2.clone());
                                m.0.insert(scenars.intersection(s1, *s2), res);
                            }
                        }
                        Ok(ExprValue::StrList(m))
                    }

                    _ => Err(Error::WrongAmpersand),
                }
            }
        }
    }
}

#[cfg(test)]
impl<T> ExprValue<T>
    where T: Eq + std::fmt::Debug
{

    /// Merge two expression values.
    /// There must not be any conflicts (value set for the same scenario in
    /// both self and right, even if the values match).
    pub fn merge(
        &mut self,
        v_right: ExprValue<T>,
        scenars: &mut AllScenarios,
    ) -> Result<(), Error> {
        for (s2, v2) in v_right.0 {
            let mut merged: Option<(Scenario, Scenario)> = None;
            for (s1, v1) in self.0.iter() {
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
                    if self.0.contains_key(&s2) {
                        Err(Error::CannotMerge)?;
                    }
                    self.0.insert(s2, v2);
                }
                Some((s1, new_s)) => {
                    let old = self.0.remove(&s1);
                    self.0.insert(new_s, old.unwrap());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::Error;
    use crate::gpr::GprFile;
    use crate::graph::NodeIndex;
    use crate::rawexpr::tests::{build_expr_list, build_expr_str};
    use crate::rawexpr::{PackageName, QualifiedName, RawExpr, SimpleName};
    use crate::scenarios::tests::{split, try_add_variable};
    use crate::scenarios::{AllScenarios, Scenario};
    use crate::values::ExprValue;
    use std::collections::HashMap;
    use ustr::Ustr;

    #[test]
    fn test_eval() -> Result<(), Error> {
        let mut gpr = GprFile::new(
            std::path::Path::new("/"),
            NodeIndex::default(),
            Ustr::from("dummy"),
        );
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
        let mut gpr = GprFile::new(
            std::path::Path::new("/"),
            NodeIndex::default(),
            Ustr::from("dummy"),
        );
        let mut scenars = AllScenarios::default();
        try_add_variable(&mut scenars, "MODE", &["debug", "optimize", "lto"])?;
        try_add_variable(&mut scenars, "CHECK", &["none", "some", "most"])?;
        let pkg = PackageName::None;
        let s0 = Scenario::default();
        let s2 = split(&mut scenars, s0, "MODE", &["debug", "optimize"]);
        let s3 = split(&mut scenars, s0, "MODE", &["lto"]);
        let s4 = split(&mut scenars, s0, "CHECK", &["some"]);
        let s5 = split(&mut scenars, s0, "CHECK", &["most", "none"]);

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
        let s5 = split(&mut scenars, s3, "CHECK", &["some"]);
        let s6 = split(&mut scenars, s3, "CHECK", &["most", "none"]);
        let s7 = split(&mut scenars, s2, "CHECK", &["some"]);
        let s8 = split(&mut scenars, s2, "CHECK", &["most", "none"]);

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

        let mut expected = ExprValue(HashMap::new());
        expected.0.insert(s7, Ustr::from("val2val4"));
        expected.0.insert(s8, Ustr::from("val2val5"));
        expected.0.insert(s5, Ustr::from("val3val4"));
        expected.0.insert(s6, Ustr::from("val3val5"));
        assert_eq!(concat_expr, ExprValue::Str(expected));

        Ok(())
    }
}
