use crate::errors::Result;
use crate::lexer::Lexer;
use crate::rawexpr::{
    RawExpr, Statement, TypeDecl, PackageDecl, VariableDecl, CaseStmt,
    WhenClause, VariableName, AttributeName, PROJECT, AttributeDecl};
use crate::rawgpr::RawGPR;
use crate::tokens::{Token, TokenKind};

pub struct Scanner<'a> {
    lex: &'a mut Lexer<'a>,
    gpr: RawGPR<'a>,
}

impl<'a> Scanner<'a> {
    pub fn new(lex: &'a mut Lexer<'a>) -> Self {
        Self {
            gpr: RawGPR::new(lex.path()),
            lex,
        }
    }

    pub fn parse(mut self) -> Result<RawGPR<'a>> {
        self.parse_file()?;
        Ok(self.gpr)
    }

    #[inline]
    fn error<T>(&self, msg: String) -> Result<T> {
        Err(self.lex.error(msg))
    }

    #[inline]
    fn peek(&self) -> &Option<Token> {
        self.lex.peek()
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    fn expect(&mut self, token: TokenKind) -> Result<()> {
        match self.lex.next() {
            Some(tk) if tk.kind == token => Ok(()),
            Some(tk) => self.error(format!("Expected {}, got {}", token, tk)),
            None => self.error("Unexpected end of file".into()),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a str> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::String(s),
                ..
            }) => Ok(std::str::from_utf8(s).unwrap()),
            Some(t) => self.error(format!("Expected String, got {}", t)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<String> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::Identifier(s),
                ..
            }) => Ok(std::str::from_utf8(s).unwrap().to_string()),
            Some(t) => self.error(format!("Expected Identifier, got {}", t)),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<VariableName> {
        let mut result = VariableName::default();
        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::Project,
                    ..
                }) => {
                    // e.g.  for source_dirs use project'source_dirs & ..
                    result.name = PROJECT.to_string();
                }
                Some(Token {
                    kind: TokenKind::Identifier(s),
                    ..
                }) => {
                    result.name = std::str::from_utf8(s).unwrap().to_string();
                },
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }

            match self.peek() {
                Some(Token {
                    kind: TokenKind::Dot,
                    ..
                }) => {
                    let _ = self.lex.next();
                    result.project = result.package;
                    result.package = Some(result.name);
                    result.name = String::new();
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn expect_attribute_reference(&mut self) -> Result<AttributeName> {
        let varname = self.expect_variable_reference()?;
        let mut qname = AttributeName {
            project: varname.project,
            package: varname.package,
            name: varname.name,
            attname: None,
            index: None,
        };

        if let Some(Token {
            kind: TokenKind::Tick,
            ..
        }) = self.peek()
        {
            // attribute ref
            let _ = self.lex.next();
            qname.attname = Some(self.expect_identifier()?);
        }
        if let Some(Token {
            kind: TokenKind::OpenParenthesis,
            ..
        }) = self.peek()
        {
            qname.index = Some(Box::new(self.parse_arg_list()?));
        }

        Ok(qname)
    }

    /// Parse a whole file
    fn parse_file(&mut self) -> Result<()> {
        loop {
            match self.peek() {
                None => return Ok(()),
                Some(Token {
                    kind: TokenKind::With,
                    ..
                }) => self.parse_with_clause()?,
                _ => self.parse_project_declaration()?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self) -> Result<()> {
        self.expect(TokenKind::With)?;

        let path = self.expect_str()?;
        self.gpr.imported.push(path.to_string());

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    /// Parses the declaration of the project, directly into self.gpr
    fn parse_project_declaration(&mut self) -> Result<()> {
        let mut body = vec![];
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::Aggregate,
                    ..
                }) => {
                    self.gpr.is_aggregate = true;
                    let _ = self.lex.next(); // consume "aggregate"
                }
                Some(Token {
                    kind: TokenKind::Library,
                    ..
                }) => {
                    self.gpr.is_library = true;
                    let _ = self.lex.next(); // consume "library"
                }
                Some(Token {
                    kind: TokenKind::Abstract,
                    ..
                }) => {
                    self.gpr.is_abstract = true;
                    let _ = self.lex.next(); // consume "abstract"
                }
                _ => break,
            }
        }

        self.expect(TokenKind::Project)?;
        self.gpr.name = self.expect_identifier()?;

        if let Some(Token {
            kind: TokenKind::Extends,
            ..
        }) = self.peek()
        {
            self.gpr.extends = Some(self.parse_project_extension()?.to_string());
        }

        self.expect(TokenKind::Is)?;

        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::End,
                    ..
                }) => break,
                Some(Token {
                    kind: TokenKind::For,
                    ..
                }) => body.push(Statement::Attribute(
                    self.parse_attribute_declaration()?)),
                Some(Token {
                    kind: TokenKind::Null,
                    ..
                }) => {}
                Some(Token {
                    kind: TokenKind::Case,
                    ..
                }) => body.push(self.parse_case_statement()?),
                Some(Token {
                    kind: TokenKind::Package,
                    ..
                }) => body.push(self.parse_package_declaration()?),
                Some(Token {
                    kind: TokenKind::Identifier(_),
                    ..
                }) => body.push(self.parse_variable_definition()?),
                Some(Token {
                    kind: TokenKind::Type,
                    ..
                }) => body.push(self.parse_type_definition()?),
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }

        self.expect(TokenKind::End)?;
        let endname = self.expect_identifier()?;
        if self.gpr.name.to_lowercase() != endname.to_lowercase() {
            return self.error(format!(
                "Expected endname {}, got {:?}", self.gpr.name, endname));
        }
        self.expect(TokenKind::Semicolon)?;

        self.gpr.body = body;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<&str> {
        self.expect(TokenKind::Extends)?;
        self.expect_str()
    }

    fn parse_type_definition(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Type)?;
        let typename = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Type(TypeDecl {
            typename,
            valid: expr.to_static_list(self.lex)?,
        }))
    }

    fn parse_package_declaration(&mut self) -> Result<Statement> {
        let mut result = PackageDecl::default();

        self.expect(TokenKind::Package)?;
        result.name = self.expect_identifier()?;

        if let Some(Token {
            kind: TokenKind::Extends,
            ..
        }) = self.peek()
        {
            self.expect(TokenKind::Extends)?;
            result.extends = Some(self.expect_variable_reference()?);
        }

        match self.lex.next() {
            None => self.error("Unexpected end of file".into())?,
            Some(Token {
                kind: TokenKind::Is,
                ..
            }) => {
                loop {
                    match self.peek() {
                        None => self.error("Unexpected end of file".into())?,
                        Some(Token {
                            kind: TokenKind::End,
                            ..
                        }) => break,
                        Some(Token {
                            kind: TokenKind::For,
                            ..
                        }) =>
                            result.body.push(Statement::Attribute(
                                self.parse_attribute_declaration()?)),
                        Some(Token {
                            kind: TokenKind::Null,
                            ..
                        }) => {}
                        Some(Token {
                            kind: TokenKind::Case,
                            ..
                        }) => result.body.push(self.parse_case_statement()?),
                        Some(Token {
                            kind: TokenKind::Identifier(_),
                            ..
                        }) => result.body.push(self.parse_variable_definition()?),
                        Some(t) => self.error(format!("Unexpected token {}", t))?,
                    }
                }

                self.expect(TokenKind::End)?;
                let endname = self.expect_identifier()?;
                if result.name != endname {
                    return self.error(format!(
                        "Expected endname {:?}, got {:?}", result.name, endname));
                }
            }
            Some(Token {
                kind: TokenKind::Renames,
                ..
            }) => {
                result.renames = Some(self.expect_variable_reference()?);
            }
            Some(t) => self.error(format!("Unexpected {}", t))?,
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Package(result))
    }

    fn parse_variable_definition(&mut self) -> Result<Statement> {
        let mut result = VariableDecl {
            name: self.expect_identifier()?,
            ..Default::default()
        };

        if let Some(Token {
            kind: TokenKind::Colon,
            ..
        }) = self.peek()
        {
            let _ = self.lex.next(); // consume ":"
            result.typename = Some(self.expect_variable_reference()?);
        }

        self.expect(TokenKind::Assign)?;
        result.expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Variable(result))
    }

    fn parse_case_statement(&mut self) -> Result<Statement> {
        let mut result = CaseStmt::default();

        self.expect(TokenKind::Case)?;
        result.varname = self.expect_variable_reference()?;
        self.expect(TokenKind::Is)?;

        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::End,
                    ..
                }) => {
                    self.expect(TokenKind::Case)?;
                    self.expect(TokenKind::Semicolon)?;
                    break;
                }
                Some(Token {
                    kind: TokenKind::When,
                    ..
                }) => {
                    let mut when = WhenClause::default();
                    loop {
                        match self.lex.next() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {
                                kind: TokenKind::String(s),
                                ..
                            }) =>
                                when.values.push(Some(
                                    std::str::from_utf8(s).unwrap().to_string()
                                )),
                            Some(Token {
                                kind: TokenKind::Others,
                                ..
                            }) => {
                                self.expect(TokenKind::Arrow)?;
                                when.values.push(None);
                                break;
                            }
                            Some(t) => self.error(format!(
                                "Unexpected token {} in when", t))?,
                        }
                        match self.lex.next() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {
                                kind: TokenKind::Pipe,
                                ..
                            }) => {}
                            Some(Token {
                                kind: TokenKind::Arrow,
                                ..
                            }) => break,
                            Some(t) => self.error(format!("Unexpected token {}", t))?,
                        }
                    }

                    loop {
                        match self.peek() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {
                                kind: TokenKind::End | TokenKind::When,
                                ..
                            }) => break,
                            Some(Token {
                                kind: TokenKind::For,
                                ..
                            }) =>
                                when.body.push(Statement::Attribute(
                                    self.parse_attribute_declaration()?
                                )),
                            Some(Token {
                                kind: TokenKind::Null,
                                ..
                            }) => {
                                let _ = self.lex.next();
                                self.expect(TokenKind::Semicolon)?;
                            }
                            Some(Token {
                                kind: TokenKind::Case,
                                ..
                            }) =>
                                when.body.push(self.parse_case_statement()?),
                            Some(Token {
                                kind: TokenKind::Identifier(_),
                                ..
                            }) =>
                                when.body.push(self.parse_variable_definition()?),
                            Some(t) => self.error(format!(
                                "Unexpected token {}", t))?,
                        }
                    }

                    result.when.push(when);
                }
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }
        Ok(Statement::Case(result))
    }

    fn parse_arg_list(&mut self) -> Result<RawExpr> {
        let mut result: RawExpr = RawExpr::Empty;

        self.expect(TokenKind::OpenParenthesis)?;

        if let Some(Token {
            kind: TokenKind::Others,
            ..
        }) = self.peek()
        {
            let _ = self.lex.next(); // consume "others"
        } else {
            loop {
                match self.peek() {
                    None => self.error("Unexpected end of file".into())?,
                    Some(Token {
                        kind: TokenKind::String(_),
                        ..
                    }) => {
                        result = result.comma(self.parse_string_expression()?);
                    }
                    Some(Token {
                        kind: TokenKind::Identifier(_),
                        ..
                    }) => {
                        let s = self.expect_attribute_reference()?;
                        result = result.comma(RawExpr::Identifier(s));
                    }
                    Some(t) => self.error(format!("Unexpected token {}", t))?,
                }

                if let Some(Token {
                    kind: TokenKind::Comma,
                    ..
                }) = self.peek()
                {
                    let _ = self.lex.next(); // consume ','
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::CloseParenthesis)?;
        Ok(result)
    }

    /// Parse a string expression.  This could either be a static string,
    ///     "value"
    /// or an actual expression to build a string
    ///     "value" & variable
    fn parse_string_expression(&mut self) -> Result<RawExpr> {
        let mut result = RawExpr::Empty;
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::String(s),
                    ..
                }) => {
                    result = result.ampersand(RawExpr::StaticString(
                        std::str::from_utf8(s).unwrap().to_string()
                    ));
                    let _ = self.lex.next(); //  consume the string
                }
                Some(Token {
                    kind: TokenKind::Identifier(_),
                    ..
                }) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let s = self.expect_attribute_reference()?;
                    result = result.ampersand(RawExpr::Identifier(s));
                }
                Some(t) => self.error(format!(
                    "Unexpected token in string expression {}", t))?,
            }

            if let Some(Token {
                kind: TokenKind::Ampersand,
                ..
            }) = self.peek()
            {
                let _ = self.lex.next(); // consume "&"
            } else {
                break;
            }
        }
        Ok(result)
    }

    fn parse_expression(&mut self) -> Result<RawExpr> {
        let mut result = RawExpr::Empty;
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::String(_),
                    ..
                }) => {
                    let r = self.parse_string_expression()?;
                    result = result.ampersand(r);
                }
                Some(Token {
                    kind: TokenKind::OpenParenthesis,
                    ..
                }) => {
                    let mut list = RawExpr::List(vec![]);
                    let _ = self.lex.next(); // consume "("
                    if let Some(Token {
                        kind: TokenKind::CloseParenthesis,
                        ..
                    }) = self.peek()
                    {
                        let _ = self.lex.next(); //  consume ")"
                                                 // Empty list
                    } else {
                        loop {
                            let s = self.parse_string_expression()?;
                            list.append(s);

                            match self.lex.next() {
                                None => self.error("Unexpected end of file".into())?,
                                Some(Token {
                                    kind: TokenKind::CloseParenthesis,
                                    ..
                                }) => break,
                                Some(Token {
                                    kind: TokenKind::Comma,
                                    ..
                                }) => {}
                                Some(t) => self.error(format!("Unexpected token {}", t))?,
                            }
                        }
                    }
                    result = result.ampersand(list);
                }
                Some(Token {
                    kind: TokenKind::Identifier(_) | TokenKind::Project,
                    ..
                }) => {
                    let att = self.expect_attribute_reference()?;
                    result = result.ampersand(RawExpr::Identifier(att));
                }
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }

            if let Some(Token {
                kind: TokenKind::Ampersand,
                ..
            }) = self.peek()
            {
                let _ = self.lex.next(); // consume "&"
            } else {
                break;
            }
        }

        Ok(result)
    }

    fn parse_attribute_declaration(&mut self) -> Result<AttributeDecl> {
        let mut result = AttributeDecl::default();

        self.expect(TokenKind::For)?;
        result.name = self.expect_str()?.to_string();

        if let Some(Token {
            kind: TokenKind::OpenParenthesis,
            ..
        }) = self.peek()
        {
            self.expect(TokenKind::OpenParenthesis)?;
            result.index = Some(self.expect_str()?.to_string());
            self.expect(TokenKind::CloseParenthesis)?;
        }

        self.expect(TokenKind::Use)?;
        result.value = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    fn do_check<F>(s: &str, check: F)
    where
        F: FnOnce(crate::errors::Result<crate::scanner::RawGPR>),
    {
        let mut lex = crate::lexer::Lexer::new_from_string(s);
        let scan = crate::scanner::Scanner::new(&mut lex);
        check(scan.parse())
    }

    fn expect_error(s: &str, msg: &str) {
        do_check(s, |g| match g {
            Err(e) => assert_eq!(e.msg, msg),
            Ok(_) => assert!(g.is_err(), "while parsing {}", s),
        })
    }

    fn expect_success<F>(s: &str, check: F)
    where
        F: FnOnce(&crate::scanner::RawGPR),
    {
        do_check(s, |g| match &g {
            Err(e) => assert!(g.is_ok(), "while parsing {}, got error {}", s, e.msg),
            Ok(g) => check(g),
        })
    }

    #[test]
    fn parse_errors() {
        expect_error("project A is", "Unexpected end of file");
    }

    #[test]
    fn parse_external() {
        expect_success(
            "project A is
                type Mode_Type is (\"debug\", \"optimize\", \"lto\");
                Mode : Mode_Type := external (\"MODE\");
            end A;",
            |_g| {
//                assert_eq!(g.types.keys().collect::<Vec<&&str>>(), vec![&"Mode_Type"]);
            },
        );
    }

//    ... tests extends
}
