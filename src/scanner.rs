use crate::errors::Error;
use crate::files::File;
use crate::graph::PathToIndexes;
use crate::lexer::Lexer;
use crate::rawexpr::{
    PackageName, QualifiedName, RawExpr, SimpleName, Statement, StringOrOthers,
    WhenClause,
};
use crate::rawgpr::RawGPR;
use crate::settings::Settings;
use crate::tokens::{Token, TokenKind};
use path_clean::PathClean;
use std::path::PathBuf;
use ustr::Ustr;

pub struct Scanner<'a> {
    lex: Lexer<'a>,
    gpr: RawGPR,
    current_pkg: PackageName, //  What are we parsing
    settings: &'a Settings,
}

impl<'a> Scanner<'a> {
    pub fn new(file: &'a mut File, settings: &'a Settings) -> Self {
        Self {
            gpr: RawGPR::new(file.path()),
            lex: Lexer::new(file),
            current_pkg: PackageName::None,
            settings,
        }
    }

    pub fn parse(
        mut self,
        path_to_id: &PathToIndexes,
    ) -> Result<RawGPR, Error> {
        let res = self.parse_file(path_to_id);
        match res {
            Err(err) => Err(self.lex.decorate_error(err)),
            Ok(()) => Ok(self.gpr),
        }
    }

    /// Get the next token, failing with error on end of file
    #[inline]
    fn safe_next(&mut self) -> Result<Token, String> {
        match self.lex.next() {
            Some(n) => Ok(n),
            None => Err("Unexpected end of file".into()),
        }
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    fn expect(&mut self, token: TokenKind) -> Result<(), String> {
        let n = self.safe_next()?;
        match n {
            tk if tk.kind == token => Ok(()),
            tk => Err(format!("Expected {}, got {}", token, tk)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<Ustr, String> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::String(s) => Ok(s),
            _ => Err(format!("Expected String, got {}", n)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or the keyword "others"
    fn expect_str_or_others(&mut self) -> Result<StringOrOthers, String> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Others => Ok(StringOrOthers::Others),
            TokenKind::String(s) => Ok(StringOrOthers::Str(s)),
            _ => Err(format!("Expected String or others, got {}", n)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.  The identifier is always lower-cased.
    fn expect_identifier(&mut self) -> Result<Ustr, String> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Identifier(s) => Ok(s),
            _ => Err(format!("Expected Identifier, got {}", n)),
        }
    }

    // Expect either "Project'" or "<name>."
    fn expect_project_name(&mut self) -> Result<Option<Ustr>, String> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Project => Ok(None),
            TokenKind::Identifier(s) => Ok(Some(s)),
            _ => Err(format!("Unexpected project name {}", n))?,
        }
    }

    /// Expects an unqualified attribute name (and optional index)
    fn expect_unqualified_attrname(&mut self) -> Result<SimpleName, String> {
        let name3 = self.expect_identifier()?;
        let args = self.parse_opt_arg_list()?;
        match args {
            None => Ok(SimpleName::new_attr(name3, None)?),
            Some(mut args) if args.len() == 1 => Ok(SimpleName::new_attr(
                name3,
                Some(StringOrOthers::Str(args.remove(0).as_static_str()?)),
            )?),
            Some(args) => {
                Err(format!("Wrong number of indexes for {:?}", args))
            }
        }
    }

    /// Expect a qualified name (variable or attribute).
    /// When we have an attribute, this also parses the index.
    fn expect_qname(&mut self) -> Result<QualifiedName, String> {
        let name1 = self.expect_project_name()?;
        match self.lex.peek() {
            TokenKind::Dot => {
                let _ = self.lex.next(); //  consume the dot
                let name2 = self.expect_identifier()?;
                match self.lex.peek() {
                    TokenKind::Dot => {
                        let _ = self.lex.next(); //  consume the dot
                        let name3 = self.expect_identifier()?;
                        Ok(QualifiedName {
                            project: name1,
                            package: PackageName::new(name2)?,
                            name: SimpleName::new_var(name3)?,
                        })
                    }
                    TokenKind::Tick => {
                        let _ = self.lex.next(); //  consume the tick
                        Ok(QualifiedName {
                            project: name1,
                            package: PackageName::new(name2)?,
                            name: self.expect_unqualified_attrname()?,
                        })
                    }
                    _ => Ok(QualifiedName::from_two(
                        name1,
                        SimpleName::new_var(name2)?,
                    )),
                }
            }
            TokenKind::Tick => {
                let _ = self.lex.next(); //  consume the dot
                let attrname = self.expect_unqualified_attrname()?;
                Ok(QualifiedName::from_two(name1, attrname))
            }
            _ => match name1 {
                None => {
                    Err("`Project'` must be followed by attribute name"
                        .to_string())?
                }
                Some(n1) => Ok(QualifiedName {
                    project: None,
                    package: PackageName::None,
                    name: SimpleName::new_var(n1)?,
                }),
            },
        }
    }

    /// Parse a whole file
    fn parse_file(&mut self, path_to_id: &PathToIndexes) -> Result<(), Error> {
        loop {
            match self.lex.peek() {
                TokenKind::EOF => return Ok(()),
                TokenKind::With => self.parse_with_clause(path_to_id)?,
                _ => self.parse_project_declaration(path_to_id)?,
            }
        }
    }

    /// Resolve relative paths for project dependencies.
    /// Optionally resolves symbolic links.
    pub fn normalize_gpr_path(&self, path: &str) -> Result<PathBuf, String> {
        let mut p = self.gpr.path.parent().unwrap().join(path);
        p.set_extension("gpr");
        if self.settings.resolve_symbolic_links {
            match std::fs::canonicalize(p) {
                Err(e) => Err(format!("{} {}", e, path)),
                Ok(p) => Ok(p),
            }
        } else {
            Ok(p.clean())
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(
        &mut self,
        path_to_id: &PathToIndexes,
    ) -> Result<(), String> {
        self.expect(TokenKind::With)?;

        let path = self.expect_str()?;
        let normalized = self.normalize_gpr_path(path.as_str())?;
        match path_to_id.get(&normalized) {
            None => {
                Err(format!("Project file {} not found", normalized.display()))?
            }
            Some(idx) => self.gpr.imported.push(idx.1),
        }

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    /// Parses the declaration of the project, directly into self.gpr
    fn parse_project_declaration(
        &mut self,
        path_to_id: &PathToIndexes,
    ) -> Result<(), String> {
        loop {
            let n = self.safe_next()?;
            match n.kind {
                TokenKind::Aggregate => self.gpr.is_aggregate = true,
                TokenKind::Library => self.gpr.is_library = true,
                TokenKind::Abstract => self.gpr.is_abstract = true,
                TokenKind::Project => break,
                _ => Err(format!("Unexpected token {}", n))?,
            }
        }

        self.gpr.name = self.expect_identifier()?;
        self.gpr.extends = if self.lex.peek() == TokenKind::Extends {
            let ext = self.parse_project_extension()?;
            let normalized = self.normalize_gpr_path(ext.as_str())?;
            Some(path_to_id[&normalized].1)
        } else {
            None
        };

        self.expect(TokenKind::Is)?;

        let mut body = Vec::new();

        loop {
            match self.lex.peek_with_line() {
                (_, TokenKind::EOF) => {
                    Err("Unexpected end of file".to_string())?
                }
                (_, TokenKind::End) => {
                    let _ = self.lex.next(); //  consume
                    let endname = self.expect_identifier()?;
                    if self.gpr.name != endname {
                        return Err(format!(
                            "Expected endname {}, got {:?}",
                            self.gpr.name, endname
                        ));
                    }
                    break;
                }
                (_, TokenKind::Null) => {}
                (line, TokenKind::For) => {
                    body.push((line, self.parse_attribute_declaration()?))
                }
                (line, TokenKind::Case) => {
                    body.push((line, self.parse_case_statement()?))
                }
                (line, TokenKind::Package) => {
                    body.push((line, self.parse_package_declaration()?))
                }
                (line, TokenKind::Identifier(_)) => {
                    body.push((line, self.parse_variable_definition()?))
                }
                (line, TokenKind::Type) => {
                    body.push((line, self.parse_type_definition()?))
                }
                _ => {
                    let n = self.safe_next()?;
                    Err(format!("Unexpected token {}", n))?;
                }
            }
        }

        self.gpr.body = body;

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<Ustr, String> {
        self.expect(TokenKind::Extends)?;
        self.expect_str()
    }

    fn parse_type_definition(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Type)?;
        let typename = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Statement::TypeDecl {
            typename,
            valid: expr,
        })
    }

    fn parse_package_declaration(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Package)?;
        let name = PackageName::new(self.expect_identifier()?)?;
        let mut extends: Option<QualifiedName> = None;
        let mut renames: Option<QualifiedName> = None;
        let mut body = Vec::new();

        self.current_pkg = name;

        loop {
            match self.lex.next() {
                None => Err("Unexpected end of file".to_string())?,
                Some(Token {
                    kind: TokenKind::Is,
                    ..
                }) => {
                    loop {
                        match self.lex.peek_with_line() {
                            (_, TokenKind::EOF) => {
                                Err("Unexpected end of file".to_string())?
                            }
                            (_, TokenKind::End) => {
                                let _ = self.lex.next(); //  consume
                                let endname = PackageName::new(
                                    self.expect_identifier()?,
                                )?;
                                if name != endname {
                                    Err(format!(
                                        "Expected endname {:?}, got {:?}",
                                        name, endname
                                    ))?;
                                }
                                break;
                            }
                            (_, TokenKind::Null) => {}
                            (line, TokenKind::For) => body.push((
                                line,
                                self.parse_attribute_declaration()?,
                            )),
                            (line, TokenKind::Case) => {
                                body.push((line, self.parse_case_statement()?))
                            }
                            (line, TokenKind::Identifier(_)) => body.push((
                                line,
                                self.parse_variable_definition()?,
                            )),
                            (_, t) => Err(format!("Unexpected token {}", t))?,
                        }
                    }
                    self.expect(TokenKind::Semicolon)?;
                    break;
                }
                Some(Token {
                    kind: TokenKind::Renames,
                    ..
                }) => {
                    renames = Some(self.expect_qname()?);
                    self.expect(TokenKind::Semicolon)?;
                    break;
                }
                Some(Token {
                    kind: TokenKind::Extends,
                    ..
                }) => extends = Some(self.expect_qname()?),
                Some(t) => Err(format!("Unexpected {}", t))?,
            }
        }

        self.current_pkg = PackageName::None;

        Ok(Statement::Package {
            name,
            renames,
            extends,
            body,
        })
    }

    fn parse_variable_definition(&mut self) -> Result<Statement, String> {
        let name = self.expect_identifier()?;
        let typename = if self.lex.peek() == TokenKind::Colon {
            let _ = self.lex.next(); // consume ":"
            Some(self.expect_qname()?)
        } else {
            None
        };

        self.expect(TokenKind::Assign)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::VariableDecl {
            name,
            typename,
            expr,
        })
    }

    fn parse_case_statement(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Case)?;
        let varname = self.expect_qname()?;
        let mut when = Vec::new();
        self.expect(TokenKind::Is)?;

        loop {
            let n = self.safe_next()?;
            match n.kind {
                TokenKind::End => {
                    self.expect(TokenKind::Case)?;
                    self.expect(TokenKind::Semicolon)?;
                    break;
                }
                TokenKind::When => {
                    let mut values = Vec::new();
                    let mut body = Vec::new();
                    loop {
                        let n = self.safe_next()?;
                        match n.kind {
                            TokenKind::EOF => {
                                Err("Unexpected end of file".to_string())?
                            }
                            TokenKind::String(s) => {
                                values.push(StringOrOthers::Str(s))
                            }
                            TokenKind::Others => {
                                self.expect(TokenKind::Arrow)?;
                                values.push(StringOrOthers::Others);
                                break;
                            }
                            _ => {
                                Err(format!("Unexpected token {} in when", n))?
                            }
                        }

                        let n = self.safe_next()?;
                        match n.kind {
                            TokenKind::EOF => {
                                Err("Unexpected end of file".to_string())?
                            }
                            TokenKind::Pipe => {}
                            TokenKind::Arrow => break,
                            _ => Err(format!("Unexpected token {}", n))?,
                        }
                    }

                    loop {
                        match self.lex.peek_with_line() {
                            (_, TokenKind::EOF) => {
                                Err("Unexpected end of file".to_string())?
                            }
                            (_, TokenKind::End | TokenKind::When) => break,
                            (line, TokenKind::For) => body.push((
                                line,
                                self.parse_attribute_declaration()?,
                            )),
                            (_, TokenKind::Null) => {
                                let _ = self.lex.next();
                                self.expect(TokenKind::Semicolon)?;
                            }
                            (line, TokenKind::Case) => {
                                body.push((line, self.parse_case_statement()?))
                            }
                            (line, TokenKind::Identifier(_)) => body.push((
                                line,
                                self.parse_variable_definition()?,
                            )),
                            (_, _) => {
                                let n = self.safe_next()?;
                                Err(format!("Unexpected token {}", n))?;
                            }
                        }
                    }

                    when.push(WhenClause { values, body });
                }
                _ => Err(format!("Unexpected token {}", n))?,
            }
        }
        Ok(Statement::Case { varname, when })
    }

    /// Parse a parenthesized expression as an attribute index, or a function
    /// argument list.
    fn parse_opt_arg_list(&mut self) -> Result<Option<Vec<RawExpr>>, String> {
        let mut result: Vec<RawExpr> = vec![];

        match self.lex.peek() {
            TokenKind::OpenParenthesis => {
                let _ = self.lex.next(); //  consume parenthesis
            }
            _ => return Ok(None),
        }

        loop {
            match self.lex.peek() {
                TokenKind::EOF => Err(
                    "Unexpected end of file, expecting closing parenthesis"
                        .to_string(),
                )?,
                TokenKind::Others => {
                    let _ = self.lex.next();
                    result.push(RawExpr::Others);
                }
                TokenKind::String(_) | TokenKind::Identifier(_) => {
                    result.push(self.parse_expression()?);
                }
                _ => {
                    let n = self.safe_next()?;
                    Err(format!("Unexpected token {}", n))?;
                }
            };

            let n = self.safe_next()?;
            match n.kind {
                TokenKind::Comma => {}
                TokenKind::CloseParenthesis => break,
                _ => Err(format!("Unexpected token {:?}", n))?,
            }
        }
        Ok(Some(result))
    }

    // The next symbol is an identifier.  It could be an attribute name with
    // optional index, a variable name, or a function call.
    fn expect_qname_or_func(&mut self) -> Result<RawExpr, String> {
        let qname = self.expect_qname()?;
        match qname {
            QualifiedName {
                project: None,
                package: PackageName::None,
                name: SimpleName::Name(_),
            } => {
                let args = self.parse_opt_arg_list()?;
                if let Some(args) = args {
                    Ok(RawExpr::FuncCall((qname, args)))
                } else {
                    Ok(RawExpr::Name(qname))
                }
            }
            _ => Ok(RawExpr::Name(qname)),
        }
    }

    fn parse_expression(&mut self) -> Result<RawExpr, String> {
        let mut result = RawExpr::Empty;
        loop {
            match self.lex.peek() {
                TokenKind::EOF => Err("Unexpected end of file".to_string())?,
                TokenKind::String(_) => {
                    let s = self.expect_str()?;
                    result = result.ampersand(RawExpr::StaticString(s));
                }
                TokenKind::Identifier(_) | TokenKind::Project => {
                    let s = self.expect_qname_or_func()?;
                    result = result.ampersand(s);
                }
                TokenKind::OpenParenthesis => {
                    let _ = self.lex.next(); // consume "("
                    let mut list = Vec::new();
                    if self.lex.peek() == TokenKind::CloseParenthesis {
                        let _ = self.lex.next(); //  consume ")",  empty list
                    } else {
                        loop {
                            list.push(Box::new(self.parse_expression()?));

                            let n = self.safe_next()?;
                            match n.kind {
                                TokenKind::CloseParenthesis => break,
                                TokenKind::Comma => {}
                                _ => Err(format!("Unexpected token {}", n))?,
                            }
                        }
                    }
                    result = result.ampersand(RawExpr::List(list));
                }
                _ => {
                    let n = self.safe_next()?;
                    Err(format!("Unexpected token {}", n))?;
                }
            }

            match self.lex.peek() {
                TokenKind::Ampersand => {
                    let _ = self.lex.next(); // consume "&"
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn parse_attribute_declaration(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::For)?;
        let name = self.expect_identifier()?;
        let index = if self.lex.peek() == TokenKind::OpenParenthesis {
            self.expect(TokenKind::OpenParenthesis)?;
            let index = Some(self.expect_str_or_others()?);
            self.expect(TokenKind::CloseParenthesis)?;
            index
        } else {
            None
        };

        self.expect(TokenKind::Use)?;
        let value = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Statement::AttributeDecl {
            name: SimpleName::new_attr(name, index)?,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::Error;
    use crate::graph::PathToIndexes;
    use crate::rawexpr::tests::build_expr_list;
    use crate::rawexpr::{
        PackageName, QualifiedName, RawExpr, SimpleName, Statement,
        StatementList, StringOrOthers,
    };
    use crate::settings::Settings;
    use ustr::Ustr;

    fn do_check<F>(s: &str, check: F)
    where
        F: FnOnce(Result<crate::scanner::RawGPR, Error>),
    {
        let mut file = crate::files::File::new_from_str(s);
        let settings = Settings::default();
        let scan = crate::scanner::Scanner::new(&mut file, &settings);
        let path_to_id: PathToIndexes = Default::default();
        let gpr = scan.parse(&path_to_id);
        check(gpr);
    }

    fn expect_error(s: &str, msg: &str) {
        do_check(s, |g| match g {
            Err(e) => assert_eq!(e.msg, msg),
            Ok(_) => assert!(g.is_err(), "while parsing {}", s),
        })
    }

    fn expect_statements(s: &str, expected: StatementList) {
        do_check(s, |g| match &g {
            Err(e) => {
                assert!(g.is_ok(), "while parsing {}, got error {}", s, e.msg)
            }
            Ok(g) => assert_eq!(g.body, expected),
        })
    }

    #[test]
    fn parse_errors() {
        expect_error("project A is", "Unexpected end of file");
    }

    #[test]
    fn parse_attribute_decl() {
        expect_statements(
            "project A is
                for Source_Files use (\"a.adb\");
                package Linker is
                   for Switches (others) use ();
                end Linker;
             end A;",
            vec![
                (
                    2,
                    Statement::AttributeDecl {
                        name: SimpleName::SourceFiles,
                        value: RawExpr::List(vec![Box::new(
                            RawExpr::StaticString(Ustr::from("a.adb")),
                        )]),
                    },
                ),
                (
                    3,
                    Statement::Package {
                        name: PackageName::Linker,
                        renames: None,
                        extends: None,
                        body: vec![(
                            4,
                            Statement::AttributeDecl {
                                name: SimpleName::Switches(
                                    StringOrOthers::Others,
                                ),
                                value: RawExpr::List(vec![]),
                            },
                        )],
                    },
                ),
            ],
        );

        expect_statements(
            "project A is
                for Source_Files use Project'Source_Files;
             end A;",
            vec![(
                2,
                Statement::AttributeDecl {
                    name: SimpleName::SourceFiles,
                    value: RawExpr::Name(QualifiedName {
                        project: None,
                        package: PackageName::None,
                        name: SimpleName::SourceFiles,
                    }),
                },
            )],
        );
    }

    #[test]
    fn parse_external() {
        expect_statements(
            "project A is
                type Mode_Type is (\"Debug\", \"Optimize\", \"lto\");
                Mode : Mode_Type := external (\"MODE\");
            end A;",
            vec![
                (
                    2,
                    Statement::TypeDecl {
                        typename: Ustr::from("mode_type"),
                        valid: build_expr_list(&["Debug", "Optimize", "lto"]),
                    },
                ),
                (
                    3,
                    Statement::VariableDecl {
                        name: Ustr::from("mode"),
                        typename: Some(QualifiedName {
                            project: None,
                            package: PackageName::None,
                            name: SimpleName::Name(Ustr::from("mode_type")),
                        }),
                        expr: RawExpr::FuncCall((
                            QualifiedName {
                                project: None,
                                package: PackageName::None,
                                name: SimpleName::Name(Ustr::from("external")),
                            },
                            vec![RawExpr::StaticString(Ustr::from("MODE"))],
                        )),
                    },
                ),
            ],
        );
    }

    //    ... tests extends
}
