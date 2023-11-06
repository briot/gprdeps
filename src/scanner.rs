use crate::errors::Result;
use crate::files::File;
use crate::lexer::Lexer;
use crate::rawexpr::{
    AttributeOrVarName, PackageName, QualifiedName, RawExpr, Statement,
    StringOrOthers, WhenClause,
};
use crate::rawgpr::RawGPR;
use crate::tokens::{Token, TokenKind};

pub struct Scanner<'a> {
    lex: Lexer<'a>,
    gpr: RawGPR,
}

impl<'a> Scanner<'a> {
    pub fn new(file: &'a File) -> Self {
        Self {
            gpr: RawGPR::new(file.path()),
            lex: Lexer::new(file),
        }
    }

    pub fn parse(mut self) -> Result<RawGPR> {
        self.parse_file()?;
        Ok(self.gpr)
    }

    /// Get the next token, failing with error on end of file
    #[inline]
    fn safe_next(&mut self) -> Result<Token<'a>> {
        match self.lex.next() {
            Some(n) => Ok(n),
            None => self.error("Unexpected end of file".into()),
        }
    }

    #[inline]
    fn error<T>(&self, msg: String) -> Result<T> {
        Err(self.lex.error(msg))
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    fn expect(&mut self, token: TokenKind) -> Result<()> {
        let n = self.safe_next()?;
        match n {
            tk if tk.kind == token => Ok(()),
            tk => self.error(format!("Expected {}, got {}", token, tk)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a str> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::String(s) => Ok(s),
            _ => self.error(format!("Expected String, got {}", n)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or the keyword "others"
    fn expect_str_or_others(&mut self) -> Result<StringOrOthers> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Others => Ok(StringOrOthers::Others),
            TokenKind::String(s) => Ok(StringOrOthers::Str(s.to_string())),
            _ => self.error(format!("Expected String or others, got {}", n)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.  The identifier is always lower-cased.
    fn expect_identifier(&mut self) -> Result<String> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Identifier(s) => Ok(s),
            _ => self.error(format!("Expected Identifier, got {}", n)),
        }
    }

    // Expect either "Project'" or "<name>."
    fn expect_project_name(&mut self) -> Result<Option<String>> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Project => Ok(None),
            TokenKind::Identifier(s) => Ok(Some(s)),
            _ => self.error(format!("Unexpected project name {}", n))?,
        }
    }

    /// Check whether we have a valid package name.
    /// When we do not have a package name, the Err returns the string itself,
    /// so that further processing can be done
    fn as_package(
        &self,
        lower: String,
    ) -> std::result::Result<PackageName, String> {
        match lower.as_str() {
            "binder" => Ok(PackageName::Binder),
            "builder" => Ok(PackageName::Builder),
            "compiler" => Ok(PackageName::Compiler),
            "ide" => Ok(PackageName::IDE),
            "linker" => Ok(PackageName::Linker),
            "naming" => Ok(PackageName::Naming),
            _ => Err(lower),
        }
    }

    /// Similar to as_package(), but returns an actual error message if the
    /// parameter is not a valid package name
    fn as_mandatory_package(&self, lower: String) -> Result<PackageName> {
        let p = self.as_package(lower);
        match p {
            Ok(p) => Ok(p),
            Err(p) => self.error(format!("Invalid package name {}", p))?,
        }
    }

    // Check whether we have a valid attribute name
    fn as_attribute(&self, lower: String) -> AttributeOrVarName {
        match lower.as_str() {
            "exec_dir" => AttributeOrVarName::ExecDir,
            "linker_options" => AttributeOrVarName::LinkerOptions,
            "main" => AttributeOrVarName::Main,
            "object_dir" => AttributeOrVarName::ObjectDir,
            "source_dirs" => AttributeOrVarName::SourceDirs,
            "source_files" => AttributeOrVarName::SourceFiles,
            "switches" => AttributeOrVarName::Switches,
            _ => AttributeOrVarName::Name(lower),
        }
    }

    /// Parse of expect_qname.
    /// Should be called after we parsed a first identifier, and the final one.
    /// name1 should be None if we had parsed "Project'"
    fn expect_qname2(
        &mut self,
        name1: Option<String>,
        name2: String,
    ) -> Result<QualifiedName> {
        match name1 {
            None => Ok(QualifiedName {
                project: name1,
                package: None,
                name: self.as_attribute(name2),
                index: self.parse_opt_arg_list()?,
            }),
            Some(n1) => match self.as_package(n1) {
                Ok(p) => Ok(QualifiedName {
                    project: None,
                    package: Some(p),
                    name: self.as_attribute(name2),
                    index: self.parse_opt_arg_list()?,
                }),
                Err(n) => Ok(QualifiedName {
                    project: Some(n),
                    package: None,
                    name: self.as_attribute(name2),
                    index: self.parse_opt_arg_list()?,
                }),
            },
        }
    }

    fn expect_qname(&mut self) -> Result<QualifiedName> {
        let name1 = self.expect_project_name()?;
        match self.lex.peek() {
            TokenKind::Dot => {
                let _ = self.lex.next(); //  consume the dot
                let name2 = self.expect_identifier()?;
                match self.lex.peek() {
                    TokenKind::Dot | TokenKind::Tick => {
                        let _ = self.lex.next(); //  consume the dot
                        let name3 = self.expect_identifier()?;
                        let p = self.as_mandatory_package(name2)?;
                        Ok(QualifiedName {
                            project: name1,
                            package: Some(p),
                            name: self.as_attribute(name3),
                            index: self.parse_opt_arg_list()?,
                        })
                    }
                    _ => self.expect_qname2(name1, name2),
                }
            }
            TokenKind::Tick => {
                let _ = self.lex.next(); //  consume the dot
                let name2 = self.expect_identifier()?;
                self.expect_qname2(name1, name2)
            }
            _ => match name1 {
                None => self.error(
                    "`Project'` must be followed by attribute name".into(),
                )?,
                Some(n1) => Ok(QualifiedName {
                    project: None,
                    package: None,
                    name: self.as_attribute(n1),
                    index: self.parse_opt_arg_list()?,
                }),
            },
        }
    }

    /// Parse a whole file
    fn parse_file(&mut self) -> Result<()> {
        loop {
            match self.lex.peek() {
                TokenKind::EOF => return Ok(()),
                TokenKind::With => self.parse_with_clause()?,
                _ => self.parse_project_declaration()?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self) -> Result<()> {
        self.expect(TokenKind::With)?;

        let path = self.expect_str()?.to_string();
        self.gpr.imported.push(path);

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    /// Parses the declaration of the project, directly into self.gpr
    fn parse_project_declaration(&mut self) -> Result<()> {
        loop {
            let n = self.safe_next()?;
            match n.kind {
                TokenKind::Aggregate => self.gpr.is_aggregate = true,
                TokenKind::Library => self.gpr.is_library = true,
                TokenKind::Abstract => self.gpr.is_abstract = true,
                TokenKind::Project => break,
                _ => self.error(format!("Unexpected token {}", n))?,
            }
        }

        self.gpr.name = self.expect_identifier()?;
        self.gpr.extends = if self.lex.peek() == TokenKind::Extends {
            Some(self.parse_project_extension()?)
        } else {
            None
        };

        self.expect(TokenKind::Is)?;

        let mut body = Vec::new();

        loop {
            match self.lex.peek() {
                TokenKind::EOF => {
                    self.error("Unexpected end of file".into())?
                }
                TokenKind::End => {
                    let _ = self.lex.next(); //  consume
                    let endname = self.expect_identifier()?;
                    if self.gpr.name != endname {
                        return self.error(format!(
                            "Expected endname {}, got {:?}",
                            self.gpr.name, endname
                        ));
                    }
                    break;
                }
                TokenKind::Null => {}
                TokenKind::For => {
                    body.push(self.parse_attribute_declaration()?)
                }
                TokenKind::Case => body.push(self.parse_case_statement()?),
                TokenKind::Package => {
                    body.push(self.parse_package_declaration()?)
                }
                TokenKind::Identifier(_) => {
                    body.push(self.parse_variable_definition()?)
                }
                TokenKind::Type => body.push(self.parse_type_definition()?),
                _ => {
                    let n = self.safe_next()?;
                    self.error(format!("Unexpected token {}", n))?;
                }
            }
        }

        self.gpr.body = body;

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<String> {
        self.expect(TokenKind::Extends)?;
        Ok(self.expect_str()?.to_string())
    }

    fn parse_type_definition(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Type)?;
        let typename = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::TypeDecl {
            typename: typename.to_string(),
            valid: expr.to_static_list(&self.lex)?,
        })
    }

    fn parse_package_declaration(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Package)?;
        let startname = self.expect_identifier()?;
        let extends = if self.lex.peek() == TokenKind::Extends {
            let _ = self.lex.next(); //  consume extends
            Some(self.expect_qname()?)
        } else {
            None
        };

        let mut renames: Option<QualifiedName> = None;
        let mut body = Vec::new();

        match self.lex.next() {
            None => self.error("Unexpected end of file".into())?,
            Some(Token {
                kind: TokenKind::Is,
                ..
            }) => {
                loop {
                    match self.lex.peek() {
                        TokenKind::EOF => {
                            self.error("Unexpected end of file".into())?
                        }
                        TokenKind::End => {
                            let _ = self.lex.next(); //  consume
                            let endname = self.expect_identifier()?;
                            if startname != endname {
                                self.error(format!(
                                    "Expected endname {:?}, got {:?}",
                                    startname, endname
                                ))?;
                            }
                            break;
                        }
                        TokenKind::Null => {}
                        TokenKind::For => {
                            body.push(self.parse_attribute_declaration()?)
                        }
                        TokenKind::Case => {
                            body.push(self.parse_case_statement()?)
                        }
                        TokenKind::Identifier(_) => {
                            body.push(self.parse_variable_definition()?)
                        }
                        t => self.error(format!("Unexpected token {}", t))?,
                    }
                }
            }
            Some(Token {
                kind: TokenKind::Renames,
                ..
            }) => renames = Some(self.expect_qname()?),
            Some(t) => self.error(format!("Unexpected {}", t))?,
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Package {
            name: self.as_mandatory_package(startname)?,
            renames,
            extends,
            body,
        })
    }

    fn parse_variable_definition(&mut self) -> Result<Statement> {
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

    fn parse_case_statement(&mut self) -> Result<Statement> {
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
                                self.error("Unexpected end of file".into())?
                            }
                            TokenKind::String(s) => {
                                values.push(StringOrOthers::Str(s.to_string()))
                            }
                            TokenKind::Others => {
                                self.expect(TokenKind::Arrow)?;
                                values.push(StringOrOthers::Others);
                                break;
                            }
                            _ => self.error(format!(
                                "Unexpected token {} in when",
                                n
                            ))?,
                        }

                        let n = self.safe_next()?;
                        match n.kind {
                            TokenKind::EOF => {
                                self.error("Unexpected end of file".into())?
                            }
                            TokenKind::Pipe => {}
                            TokenKind::Arrow => break,
                            _ => {
                                self.error(format!("Unexpected token {}", n))?
                            }
                        }
                    }

                    loop {
                        match self.lex.peek() {
                            TokenKind::EOF => {
                                self.error("Unexpected end of file".into())?
                            }
                            TokenKind::End | TokenKind::When => break,
                            TokenKind::For => {
                                body.push(self.parse_attribute_declaration()?)
                            }
                            TokenKind::Null => {
                                let _ = self.lex.next();
                                self.expect(TokenKind::Semicolon)?;
                            }
                            TokenKind::Case => {
                                body.push(self.parse_case_statement()?)
                            }
                            TokenKind::Identifier(_) => {
                                body.push(self.parse_variable_definition()?)
                            }
                            _ => {
                                let n = self.safe_next()?;
                                self.error(format!("Unexpected token {}", n))?;
                            }
                        }
                    }

                    when.push(WhenClause { values, body });
                }
                _ => self.error(format!("Unexpected token {}", n))?,
            }
        }
        Ok(Statement::Case { varname, when })
    }

    /// Parse a parenthesized expression as an attribute index, or a function
    /// argument list.
    fn parse_opt_arg_list(&mut self) -> Result<Option<Vec<RawExpr>>> {
        let mut result: Vec<RawExpr> = vec![];

        match self.lex.peek() {
            TokenKind::OpenParenthesis => {
                let _ = self.lex.next(); //  consume parenthesis
            }
            _ => return Ok(None),
        }

        loop {
            match self.lex.peek() {
                TokenKind::EOF => self.error(
                    "Unexpected end of file, expecting closing parenthesis"
                        .into(),
                )?,
                TokenKind::Others => {
                    let _ = self.lex.next();
                    result.push(RawExpr::Others);
                }
                TokenKind::String(_) | TokenKind::Identifier(_) => {
                    result.push(self.parse_string_expression()?);
                }
                _ => {
                    let n = self.safe_next()?;
                    self.error(format!("Unexpected token {}", n))?;
                }
            };

            let n = self.safe_next()?;
            match n.kind {
                TokenKind::Comma => {}
                TokenKind::CloseParenthesis => break,
                _ => self.error(format!("Unexpected token {:?}", n))?,
            }
        }
        Ok(Some(result))
    }

    /// Parse a string expression.  This could either be a static string,
    ///     "value"
    /// or an actual expression to build a string
    ///     "value" & variable
    fn parse_string_expression(&mut self) -> Result<RawExpr> {
        let mut result = RawExpr::Empty;
        loop {
            match self.lex.peek() {
                TokenKind::EOF => {
                    self.error("Unexpected end of file".into())?
                }
                TokenKind::String(_) => {
                    let s = self.expect_str()?.to_string();
                    result = result.ampersand(RawExpr::StaticString(s));
                }
                TokenKind::Identifier(_) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let att = RawExpr::Name(self.expect_qname()?);
                    result = result.ampersand(att);
                }
                _ => {
                    let n = self.safe_next()?;
                    self.error(format!(
                        "Unexpected token in string expression {}",
                        n
                    ))?;
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

    fn parse_expression(&mut self) -> Result<RawExpr> {
        let mut result = RawExpr::Empty;
        loop {
            match self.lex.peek() {
                TokenKind::EOF => {
                    self.error("Unexpected end of file".into())?
                }
                TokenKind::String(_) => {
                    let r = self.parse_string_expression()?;
                    result = result.ampersand(r);
                }
                TokenKind::OpenParenthesis => {
                    let mut list = RawExpr::List(vec![]);
                    let _ = self.lex.next(); // consume "("
                    if self.lex.peek() == TokenKind::CloseParenthesis {
                        let _ = self.lex.next(); //  consume ")",  empty list
                    } else {
                        loop {
                            let s = self.parse_string_expression()?;
                            list.append(s);

                            let n = self.safe_next()?;
                            match n.kind {
                                TokenKind::CloseParenthesis => break,
                                TokenKind::Comma => {}
                                _ => self
                                    .error(format!("Unexpected token {}", n))?,
                            }
                        }
                    }
                    result = result.ampersand(list);
                }
                TokenKind::Identifier(_) | TokenKind::Project => {
                    let att = RawExpr::Name(self.expect_qname()?);
                    result = result.ampersand(att);
                }
                _ => {
                    let n = self.safe_next()?;
                    self.error(format!("Unexpected token {}", n))?;
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

    fn parse_attribute_declaration(&mut self) -> Result<Statement> {
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
            name: self.as_attribute(name),
            index,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::rawexpr::{
        AttributeOrVarName, PackageName, QualifiedName, RawExpr, Statement,
        StringOrOthers,
    };

    fn do_check<F>(s: &str, check: F)
    where
        F: FnOnce(crate::errors::Result<crate::scanner::RawGPR>),
    {
        let file = crate::files::File::new_from_str(s);
        let scan = crate::scanner::Scanner::new(&file);
        let gpr = scan.parse();
        check(gpr);
    }

    fn expect_error(s: &str, msg: &str) {
        do_check(s, |g| match g {
            Err(e) => assert_eq!(e.msg, msg),
            Ok(_) => assert!(g.is_err(), "while parsing {}", s),
        })
    }

    fn expect_statements(s: &str, expected: Vec<Statement>) {
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
                Statement::AttributeDecl {
                    name: AttributeOrVarName::SourceFiles,
                    index: None,
                    value: RawExpr::List(vec![Box::new(
                        RawExpr::StaticString("a.adb".to_string()),
                    )]),
                },
                Statement::Package {
                    name: PackageName::Linker,
                    renames: None,
                    extends: None,
                    body: vec![Statement::AttributeDecl {
                        name: AttributeOrVarName::Switches,
                        index: Some(StringOrOthers::Others),
                        value: RawExpr::List(vec![]),
                    }],
                },
            ],
        );

        expect_statements(
            "project A is
                for Source_Files use Project'Source_Files;
             end A;",
            vec![Statement::AttributeDecl {
                name: AttributeOrVarName::SourceFiles,
                index: None,
                value: RawExpr::Name(QualifiedName {
                    project: None,
                    package: None,
                    name: AttributeOrVarName::SourceFiles,
                    index: None,
                }),
            }],
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
                Statement::TypeDecl {
                    typename: "mode_type".to_string(),
                    valid: vec![
                        "Debug".to_string(),
                        "Optimize".to_string(),
                        "lto".to_string(),
                    ],
                },
                Statement::VariableDecl {
                    name: "mode".to_string(),
                    typename: Some(QualifiedName {
                        project: None,
                        package: None,
                        name: AttributeOrVarName::Name("mode_type".to_string()),
                        index: None,
                    }),
                    expr: RawExpr::Name(QualifiedName {
                        project: None,
                        package: None,
                        name: AttributeOrVarName::Name("external".to_string()),
                        index: Some(vec![RawExpr::StaticString(
                            "MODE".to_string(),
                        )]),
                    }),
                },
            ],
        );
    }

    //    ... tests extends
}
