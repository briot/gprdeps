use crate::ada_lexer::AdaLexer;
use crate::base_lexer::BaseScanner;
use crate::errors::Error;
use crate::graph::NodeIndex;
use crate::rawexpr::{
    PackageName, QualifiedName, RawExpr, SimpleName, Statement, StringOrOthers,
    WhenClause,
};
use crate::rawgpr::RawGPR;
use crate::settings::Settings;
use crate::tokens::{Token, TokenKind};
use path_clean::PathClean;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use ustr::Ustr;

pub type PathToIndexes = HashMap<std::path::PathBuf, NodeIndex>;

pub struct GprScanner<'a> {
    base: BaseScanner<AdaLexer<'a>>,
    gpr: RawGPR,
    current_pkg: PackageName, //  What are we parsing
    settings: &'a Settings,
}

impl<'a> GprScanner<'a> {
    /// Parse a whole file
    pub fn parse(
        lex: AdaLexer<'a>,
        path: &Path,
        path_to_id: &PathToIndexes,
        settings: &'a Settings,
    ) -> Result<RawGPR, Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
            gpr: RawGPR::new(path),
            current_pkg: PackageName::None,
            settings,
        };

        loop {
            match scan.base.peek() {
                TokenKind::EndOfFile => break,
                TokenKind::With => scan.parse_with_clause(path_to_id),
                _ => scan.parse_project_declaration(path_to_id),
            }
            .map_err(|e| scan.base.error_with_location(e))?;
        }
        Ok(scan.gpr)
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or the keyword "others"
    fn expect_str_or_others(&mut self) -> Result<StringOrOthers, Error> {
        let n = self.base.safe_next()?;
        match n.kind {
            TokenKind::Others => Ok(StringOrOthers::Others),
            TokenKind::String(s) => Ok(StringOrOthers::Str(s)),
            _ => Err(Error::wrong_token("String or others", n)),
        }
    }

    // Expect either "Project'" or "<name>."
    fn expect_project_name(&mut self) -> Result<Option<Ustr>, Error> {
        let n = self.base.safe_next()?;
        match n.kind {
            TokenKind::Project => Ok(None),
            TokenKind::Identifier(s) => Ok(Some(s)),
            _ => Err(Error::wrong_token("project name", n)),
        }
    }

    /// Expects an unqualified attribute name (and optional index)
    fn expect_unqualified_attrname(&mut self) -> Result<SimpleName, Error> {
        let name3 = self.base.expect_identifier()?;
        let insensitive = SimpleName::is_case_insensitive(&name3);
        let args = self.parse_opt_arg_list()?;
        match args {
            None => Ok(SimpleName::new_attr(name3, None)?),
            Some(mut args) if args.len() == 1 => Ok(SimpleName::new_attr(
                name3,
                Some(StringOrOthers::Str(if insensitive.0 {
                    Ustr::from(
                        &args.remove(0).into_static_str()?.to_lowercase(),
                    )
                } else {
                    args.remove(0).into_static_str()?
                })),
            )?),
            Some(_) => Err(Error::WrongIndexes(name3)),
        }
    }

    /// Expect a qualified name (variable or attribute).
    /// When we have an attribute, this also parses the index.
    fn expect_qname(&mut self) -> Result<QualifiedName, Error> {
        let name1 = self.expect_project_name()?;
        match self.base.peek() {
            TokenKind::Dot => {
                let _ = self.base.next_token(); //  consume the dot
                let name2 = self.base.expect_identifier()?;
                match self.base.peek() {
                    TokenKind::Dot => {
                        let _ = self.base.next_token(); //  consume the dot
                        let name3 = self.base.expect_identifier()?;
                        Ok(QualifiedName {
                            project: name1,
                            package: PackageName::new(name2)?,
                            name: SimpleName::new_var(name3),
                        })
                    }
                    TokenKind::Tick => {
                        let _ = self.base.next_token(); //  consume the tick
                        Ok(QualifiedName {
                            project: name1,
                            package: PackageName::new(name2)?,
                            name: self.expect_unqualified_attrname()?,
                        })
                    }
                    _ => Ok(QualifiedName::from_two(
                        name1,
                        SimpleName::new_var(name2),
                    )),
                }
            }
            TokenKind::Tick => {
                let _ = self.base.next_token(); //  consume the dot
                let attrname = self.expect_unqualified_attrname()?;
                Ok(QualifiedName::from_two(name1, attrname))
            }
            _ => match name1 {
                None => Err(Error::MissingAttributeNameAfterProject)?,
                Some(n1) => Ok(QualifiedName {
                    project: None,
                    package: PackageName::None,
                    name: SimpleName::new_var(n1),
                }),
            },
        }
    }

    /// Resolve relative paths for project dependencies.
    /// Optionally resolves symbolic links.
    fn normalize_gpr_path(&self, path: &str) -> Result<PathBuf, Error> {
        let mut p = self.gpr.path.parent().unwrap().join(path);
        p.set_extension("gpr");
        if self.settings.resolve_symbolic_links {
            std::fs::canonicalize(&p).map_err(|e| Error::IoWithPath(e, p))
        } else {
            Ok(p.clean())
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(
        &mut self,
        path_to_id: &PathToIndexes,
    ) -> Result<(), Error> {
        self.base.expect(TokenKind::With)?;

        let path = self.base.expect_str()?;
        let normalized = self.normalize_gpr_path(path.as_str())?;
        match path_to_id.get(&normalized) {
            None => Err(Error::not_found(normalized.display()))?,
            Some(idx) => self.gpr.imported.push(*idx),
        }

        self.base.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    /// Parses the declaration of the project, directly into self.gpr
    fn parse_project_declaration(
        &mut self,
        path_to_id: &PathToIndexes,
    ) -> Result<(), Error> {
        loop {
            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::Aggregate => self.gpr.is_aggregate = true,
                TokenKind::Library => self.gpr.is_library = true,
                TokenKind::Abstract => self.gpr.is_abstract = true,
                TokenKind::Project => break,
                _ => Err(Error::wrong_token(
                    "Aggregate|Library|Abstract|Project",
                    n,
                ))?,
            }
        }

        self.gpr.name = self.base.expect_identifier()?;
        self.gpr.extends = if self.base.peek() == TokenKind::Extends {
            let ext = self.parse_project_extension()?;
            let normalized = self.normalize_gpr_path(ext.as_str())?;
            Some(path_to_id[&normalized])
        } else {
            None
        };

        self.base.expect(TokenKind::Is)?;

        let mut body = Vec::new();

        loop {
            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::End => {
                    let endname = self.base.expect_identifier()?;
                    if self.gpr.name != endname {
                        Err(Error::MismatchEndName(endname, self.gpr.name))?;
                    }
                    break;
                }
                TokenKind::Null => {}
                TokenKind::For => {
                    body.push((n.line, self.parse_attribute_declaration()?))
                }
                TokenKind::Case => {
                    body.push((n.line, self.parse_case_statement()?))
                }
                TokenKind::Package => {
                    body.push((n.line, self.parse_package_declaration()?))
                }
                TokenKind::Identifier(name) => {
                    body.push((n.line, self.parse_variable_definition(name)?))
                }
                TokenKind::Type => {
                    body.push((n.line, self.parse_type_definition()?))
                }
                _ => Err(Error::wrong_token(
                    "end|for|case|package|identifier|type",
                    n,
                ))?,
            }
        }

        self.gpr.body = body;

        self.base.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<Ustr, Error> {
        self.base.expect(TokenKind::Extends)?;
        self.base.expect_str()
    }

    fn parse_type_definition(&mut self) -> Result<Statement, Error> {
        let typename = self.base.expect_identifier()?;
        self.base.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.base.expect(TokenKind::Semicolon)?;
        Ok(Statement::TypeDecl {
            typename,
            valid: expr,
        })
    }

    fn parse_package_declaration(&mut self) -> Result<Statement, Error> {
        let startname = self.base.expect_identifier()?;
        let name = PackageName::new(startname)?;
        let mut extends: Option<QualifiedName> = None;
        let mut renames: Option<QualifiedName> = None;
        let mut body = Vec::new();

        self.current_pkg = name;

        loop {
            match self.base.next_token() {
                None => Err(Error::UnexpectedEOF)?,
                Some(Token {
                    kind: TokenKind::Is,
                    ..
                }) => {
                    loop {
                        let n = self.base.safe_next()?;
                        match n.kind {
                            TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                            TokenKind::End => {
                                let endname = self.base.expect_identifier()?;
                                if startname != endname {
                                    Err(Error::MismatchEndName(
                                        endname, startname,
                                    ))?;
                                }
                                break;
                            }
                            TokenKind::Null => {}
                            TokenKind::For => body.push((
                                n.line,
                                self.parse_attribute_declaration()?,
                            )),
                            TokenKind::Case => body
                                .push((n.line, self.parse_case_statement()?)),
                            TokenKind::Identifier(name) => body.push((
                                n.line,
                                self.parse_variable_definition(name)?,
                            )),
                            t => Err(Error::wrong_token(
                                "end|null|for|case|identifier",
                                t,
                            ))?,
                        }
                    }
                    self.base.expect(TokenKind::Semicolon)?;
                    break;
                }
                Some(Token {
                    kind: TokenKind::Renames,
                    ..
                }) => {
                    renames = Some(self.expect_qname()?);
                    self.base.expect(TokenKind::Semicolon)?;
                    break;
                }
                Some(Token {
                    kind: TokenKind::Extends,
                    ..
                }) => extends = Some(self.expect_qname()?),
                Some(t) => Err(Error::wrong_token("is|renames|extends", t))?,
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

    fn parse_variable_definition(
        &mut self,
        name: Ustr,
    ) -> Result<Statement, Error> {
        let typename = if self.base.peek() == TokenKind::Colon {
            let _ = self.base.next_token(); // consume ":"
            Some(self.expect_qname()?)
        } else {
            None
        };

        self.base.expect(TokenKind::Assign)?;
        let expr = self.parse_expression()?;
        self.base.expect(TokenKind::Semicolon)?;

        Ok(Statement::VariableDecl {
            name,
            typename,
            expr,
        })
    }

    fn parse_case_statement(&mut self) -> Result<Statement, Error> {
        let varname = self.expect_qname()?;
        let mut when = Vec::new();
        self.base.expect(TokenKind::Is)?;

        loop {
            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::End => {
                    self.base.expect(TokenKind::Case)?;
                    self.base.expect(TokenKind::Semicolon)?;
                    break;
                }
                TokenKind::When => {
                    let mut values = Vec::new();
                    let mut body = Vec::new();
                    loop {
                        let n = self.base.safe_next()?;
                        match n.kind {
                            TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                            TokenKind::String(s) => {
                                values.push(StringOrOthers::Str(s))
                            }
                            TokenKind::Others => {
                                self.base.expect(TokenKind::Arrow)?;
                                values.push(StringOrOthers::Others);
                                break;
                            }
                            _ => Err(Error::wrong_token("string|others", n))?,
                        }

                        let n = self.base.safe_next()?;
                        match n.kind {
                            TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                            TokenKind::Pipe => {}
                            TokenKind::Arrow => break,
                            _ => Err(Error::wrong_token("| or =>", n))?,
                        }
                    }

                    loop {
                        if matches!(
                            self.base.peek(),
                            TokenKind::End | TokenKind::When
                        ) {
                            break;
                        }

                        let n = self.base.safe_next()?;
                        match n.kind {
                            TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                            TokenKind::For => body.push((
                                n.line,
                                self.parse_attribute_declaration()?,
                            )),
                            TokenKind::Null => {
                                self.base.expect(TokenKind::Semicolon)?;
                            }
                            TokenKind::Case => body
                                .push((n.line, self.parse_case_statement()?)),
                            TokenKind::Identifier(name) => body.push((
                                n.line,
                                self.parse_variable_definition(name)?,
                            )),
                            _ => {
                                Err(Error::wrong_token(
                                    "end|when|null|case|identifier",
                                    n,
                                ))?;
                            }
                        }
                    }

                    when.push(WhenClause { values, body });
                }
                _ => Err(Error::wrong_token("end|when", n))?,
            }
        }
        Ok(Statement::Case { varname, when })
    }

    /// Parse a parenthesized expression as an attribute index, or a function
    /// argument list.
    fn parse_opt_arg_list(&mut self) -> Result<Option<Vec<RawExpr>>, Error> {
        let mut result: Vec<RawExpr> = vec![];

        match self.base.peek() {
            TokenKind::OpenParenthesis => {
                let _ = self.base.next_token(); //  consume parenthesis
            }
            _ => return Ok(None),
        }

        loop {
            match self.base.peek() {
                TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                TokenKind::Others => {
                    let _ = self.base.next_token();
                    result.push(RawExpr::Others);
                }
                TokenKind::String(_) | TokenKind::Identifier(_) => {
                    result.push(self.parse_expression()?);
                }
                _ => {
                    let n = self.base.safe_next()?;
                    Err(Error::wrong_token("others|string", n))?;
                }
            };

            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::Comma => {}
                TokenKind::CloseParenthesis => break,
                _ => Err(Error::wrong_token(")|,", n))?,
            }
        }
        Ok(Some(result))
    }

    // The next symbol is an identifier.  It could be an attribute name with
    // optional index, a variable name, or a function call.
    fn expect_qname_or_func(&mut self) -> Result<RawExpr, Error> {
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

    fn parse_expression(&mut self) -> Result<RawExpr, Error> {
        let mut result = RawExpr::Empty;
        loop {
            match self.base.peek() {
                TokenKind::EndOfFile => Err(Error::UnexpectedEOF)?,
                TokenKind::String(_) => {
                    let s = self.base.expect_str()?;
                    result = result.ampersand(RawExpr::Str(s));
                }
                TokenKind::Identifier(_) | TokenKind::Project => {
                    let s = self.expect_qname_or_func()?;
                    result = result.ampersand(s);
                }
                TokenKind::OpenParenthesis => {
                    let _ = self.base.next_token(); // consume "("
                    let mut list = Vec::new();
                    if self.base.peek() == TokenKind::CloseParenthesis {
                        //  consume ")",  empty list
                        let _ = self.base.next_token();
                    } else {
                        loop {
                            list.push(self.parse_expression()?);
                            let n = self.base.safe_next()?;
                            match n.kind {
                                TokenKind::CloseParenthesis => break,
                                TokenKind::Comma => {}
                                _ => Err(Error::wrong_token(
                                    "closing parenthesis",
                                    n,
                                ))?,
                            }
                        }
                    }
                    result = result.ampersand(RawExpr::List(list));
                }
                _ => {
                    let n = self.base.safe_next()?;
                    Err(Error::wrong_token("string|identifier|(", n))?;
                }
            }

            match self.base.peek() {
                TokenKind::Ampersand => {
                    let _ = self.base.next_token(); // consume "&"
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn parse_attribute_declaration(&mut self) -> Result<Statement, Error> {
        let name = self.base.expect_identifier()?;
        let insensitive = SimpleName::is_case_insensitive(&name);

        let index = if self.base.peek() == TokenKind::OpenParenthesis {
            self.base.expect(TokenKind::OpenParenthesis)?;
            let index = self.expect_str_or_others()?;
            self.base.expect(TokenKind::CloseParenthesis)?;
            match (index, insensitive.0) {
                (StringOrOthers::Str(s), true) => Some(StringOrOthers::Str(
                    Ustr::from(&s.as_str().to_lowercase()),
                )),
                (s, _) => Some(s),
            }
        } else {
            None
        };

        self.base.expect(TokenKind::Use)?;
        let value = self.parse_expression()?;
        self.base.expect(TokenKind::Semicolon)?;
        Ok(Statement::AttributeDecl {
            name: SimpleName::new_attr(name, index)?,
            value: if insensitive.1 {
                value.to_lowercase()
            } else {
                value
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
    use crate::errors::Error;
    use crate::gpr_scanner::{GprScanner, PathToIndexes};
    use crate::rawexpr::tests::build_expr_list;
    use crate::rawexpr::{
        PackageName, QualifiedName, RawExpr, SimpleName, Statement,
        StatementList, StringOrOthers,
    };
    use crate::rawgpr::RawGPR;
    use crate::settings::Settings;
    use std::path::Path;
    use ustr::Ustr;

    fn do_check<F>(s: &str, check: F)
    where
        F: FnOnce(Result<RawGPR, Error>),
    {
        let mut file = crate::files::File::new_from_str(s);
        let settings = Settings::default();
        let options = AdaLexerOptions {
            kw_aggregate: true,
            kw_body: false,
        };
        let lex = AdaLexer::new(&mut file, options);
        let path_to_id: PathToIndexes = Default::default();
        let gpr =
            GprScanner::parse(lex, Path::new("memory"), &path_to_id, &settings);
        check(gpr);
    }

    fn expect_error(s: &str, msg: &str) {
        do_check(s, |g| match g {
            Err(e) => assert_eq!(e.to_string(), msg),
            Ok(_) => assert!(g.is_err(), "while parsing {}", s),
        })
    }

    fn expect_statements(s: &str, expected: StatementList) {
        do_check(s, |g| match &g {
            Err(e) => {
                assert!(g.is_ok(), "while parsing {}, got error {}", s, e)
            }
            Ok(g) => assert_eq!(g.body, expected),
        })
    }

    #[test]
    fn parse_errors() {
        expect_error("project A is", ":memory::1 Unexpected end of file");
    }

    #[test]
    fn parse_attribute_decl() {
        expect_statements(
            "project A is
                for Source_Files use (\"a.adb\");
                for Languages use (\"ADA\", \"C\");
                package Linker is
                   for Switches (\"ADA\") use ();
                   for Switches (others) use ();
                end Linker;
             end A;",
            vec![
                (
                    2,
                    Statement::AttributeDecl {
                        name: SimpleName::SourceFiles,
                        value: RawExpr::List(vec![RawExpr::Str(Ustr::from(
                            "a.adb",
                        ))]),
                    },
                ),
                (
                    3,
                    Statement::AttributeDecl {
                        name: SimpleName::Languages,
                        value: RawExpr::List(vec![
                            RawExpr::Str(Ustr::from("ada")),
                            RawExpr::Str(Ustr::from("c")),
                        ]),
                    },
                ),
                (
                    4,
                    Statement::Package {
                        name: PackageName::Linker,
                        renames: None,
                        extends: None,
                        body: vec![
                            (
                                5,
                                Statement::AttributeDecl {
                                    name: SimpleName::Switches(
                                        StringOrOthers::Str(Ustr::from("ada")),
                                    ),
                                    value: RawExpr::List(vec![]),
                                },
                            ),
                            (
                                6,
                                Statement::AttributeDecl {
                                    name: SimpleName::Switches(
                                        StringOrOthers::Others,
                                    ),
                                    value: RawExpr::List(vec![]),
                                },
                            ),
                        ],
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
                            vec![RawExpr::Str(Ustr::from("MODE"))],
                        )),
                    },
                ),
            ],
        );
    }

    //    ... tests extends
}
