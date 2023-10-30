use crate::errors::Result;
use crate::lexer::Lexer;
use crate::tokens::{Token, TokenKind};
use crate::rawexpr::RawExpr;
use crate::rawgpr::{RawGPR, ProjectDecl};

type ParserResult = Result<()>;

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
    fn peek(&self) -> &Option<Token<'a>> {
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
    fn expect_str(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::String(s),
                ..
            }) => Ok(s),
            Some(t) => self.error(format!("Expected String, got {}", t)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::Identifier(s),
                ..
            }) => Ok(s),
            Some(t) => self.error(format!("Expected Identifier, got {}", t)),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<String> {
        let mut varname = String::new();
        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::Project,
                    ..
                }) => {
                    // e.g.  for source_dirs use project'source_dirs & ..
                    varname.push_str("project");
                }
                Some(Token {
                    kind: TokenKind::Identifier(s),
                    ..
                }) => varname.push_str(std::str::from_utf8(s).unwrap()),
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }

            match self.peek() {
                Some(Token {
                    kind: TokenKind::Dot,
                    ..
                }) => {
                    let _ = self.lex.next();
                    varname.push('.');
                }
                _ => break,
            }
        }
        Ok(varname)
    }

    fn expect_attribute_reference(&mut self) -> Result<String> {
        let mut varname = self.expect_variable_reference()?;

        if let Some(Token {
            kind: TokenKind::Tick,
            ..
        }) = self.peek()
        {
            // attribute ref
            let _ = self.lex.next();
            let attname = self.expect_identifier()?;
            varname.push('\'');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }
        if let Some(Token {
            kind: TokenKind::OpenParenthesis,
            ..
        }) = self.peek()
        {
            self.parse_arg_list()?;
        }

        Ok(varname)
    }

    /// Parse a whole file
    fn parse_file(&mut self) -> ParserResult {
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
    fn parse_with_clause(&mut self) -> ParserResult {
        self.expect(TokenKind::With)?;

        let path = self.expect_str()?;
        self.gpr.imported.push(path);

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_declaration(&mut self) -> Result<ProjectDecl> {
        let mut result: ProjectDecl = ProjectDecl::default();

        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token { kind: TokenKind::Aggregate, .. }) => {
                    result.is_aggregate = true;
                    let _ = self.lex.next(); // consume "aggregate"
                },
                Some(Token { kind: TokenKind::Library, .. }) => {
                    result.is_library = true;
                    let _ = self.lex.next(); // consume "library"
                },
                Some(Token { kind: TokenKind::Abstract, .. }) => {
                    result.is_abstract = true;
                    let _ = self.lex.next(); // consume "abstract"
                },
                Some(Token { kind: TokenKind::Project, .. }) => {
                    break;
                },
                _ => { self.expect(TokenKind::Project)? },
            }
        }

        result.name = std::str::from_utf8(self.expect_identifier()?).unwrap();

        if let Some(Token {
            kind: TokenKind::Extends,
            ..
        }) = self.peek()
        {
            result.extends = Some(self.parse_project_extension()?);
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
                }) => self.parse_attribute_declaration()?,
                Some(Token {
                    kind: TokenKind::Null,
                    ..
                }) => {}
                Some(Token {
                    kind: TokenKind::Case,
                    ..
                }) => self.parse_case_statement()?,
                Some(Token {
                    kind: TokenKind::Package,
                    ..
                }) => self.parse_package_declaration()?,
                Some(Token {
                    kind: TokenKind::Identifier(_),
                    ..
                }) => self.parse_variable_definition()?,
                Some(Token {
                    kind: TokenKind::Type,
                    ..
                }) => self.parse_type_definition(scenarios)?,
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }

        self.expect(TokenKind::End)?;
        let endname = std::str::from_utf8(self.expect_identifier()?).unwrap();
        if name.to_lowercase() != endname.to_lowercase() {
            return self.error(format!("Expected endname {:?}, got {:?}", name, endname));
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<&str> {
        self.expect(TokenKind::Extends)?;
        Ok(std::str::from_utf8(self.expect_str()?).unwrap())
    }

    fn parse_type_definition(
        &mut self,
        _scenarios: &mut AllScenarios,
    ) -> ParserResult {
        self.expect(TokenKind::Type)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        let n = std::str::from_utf8(name).unwrap();
        self.gpr.types.insert(n, expr);
        Ok(())
    }

    fn parse_package_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::Package)?;
        let name = self.expect_identifier()?;

        if let Some(Token {
            kind: TokenKind::Extends,
            ..
        }) = self.peek()
        {
            self.expect(TokenKind::Extends)?;
            let _extended = self.expect_variable_reference()?;
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
                        }) => self.parse_attribute_declaration()?,
                        Some(Token {
                            kind: TokenKind::Null,
                            ..
                        }) => {}
                        Some(Token {
                            kind: TokenKind::Case,
                            ..
                        }) => self.parse_case_statement()?,
                        Some(Token {
                            kind: TokenKind::Identifier(_),
                            ..
                        }) => self.parse_variable_definition()?,
                        Some(t) => self.error(format!("Unexpected token {}", t))?,
                    }
                }

                self.expect(TokenKind::End)?;
                let endname = self.expect_identifier()?;
                if name != endname {
                    return self.error(format!("Expected endname {:?}, got {:?}", name, endname));
                }
            }
            Some(Token {
                kind: TokenKind::Renames,
                ..
            }) => {
                let _orig = self.expect_variable_reference();
            }
            Some(t) => self.error(format!("Unexpected {}", t))?,
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(())
    }

    fn parse_variable_definition(&mut self) -> ParserResult {
        let name = self.expect_identifier()?;
        let mut typ: Option<String> = None;  //  qualified type name

        if let Some(Token {
            kind: TokenKind::Colon,
            ..
        }) = self.peek()
        {
            let _ = self.lex.next(); // consume ":"
            typ = Some(self.expect_variable_reference()?);
        }

        self.expect(TokenKind::Assign)?;
        let e = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        match typ {
            Some(t) => {
                println!("MANU variable {} with type {} declared {:?}",
                   std::str::from_utf8(name).unwrap(), t, e);
            },
            None => {},
        }
        Ok(())
    }

    fn parse_case_statement(&mut self) -> ParserResult {
        self.expect(TokenKind::Case)?;
        let _varname = self.expect_variable_reference();
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
                    loop {
                        match self.lex.next() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {
                                kind: TokenKind::String(_s),
                                ..
                            }) => {}
                            Some(Token {
                                kind: TokenKind::Others,
                                ..
                            }) => {
                                self.expect(TokenKind::Arrow)?;
                                break;
                            }
                            Some(t) => self.error(format!("Unexpected token {} in when", t))?,
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
                            }) => self.parse_attribute_declaration()?,
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
                            }) => self.parse_case_statement()?,
                            Some(Token {
                                kind: TokenKind::Identifier(_),
                                ..
                            }) => self.parse_variable_definition()?,
                            Some(t) => self.error(format!("Unexpected token {}", t))?,
                        }
                    }
                }
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }

        Ok(())
    }

    fn parse_arg_list(&mut self) -> ParserResult {
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
                        let _args = self.parse_string_expression();
                    }
                    Some(Token {
                        kind: TokenKind::Identifier(_),
                        ..
                    }) => {
                        let _id = self.expect_attribute_reference()?;
                        if let Some(Token {
                            kind: TokenKind::OpenParenthesis,
                            ..
                        }) = self.peek()
                        {
                            self.parse_arg_list()?;
                        }
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
        Ok(())
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
                        std::str::from_utf8(s).unwrap().to_string())
                    );
                    let _ = self.lex.next();   //  consume the string
                }
                Some(Token {
                    kind: TokenKind::Identifier(_),
                    ..
                }) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let s = self.expect_attribute_reference()?;
                    result = result.ampersand(RawExpr::Identifier(s));
                }
                Some(t) =>
                    self.error(format!(
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

    fn parse_attribute_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::For)?;
        let _attname = self.expect_str();

        if let Some(Token {
            kind: TokenKind::OpenParenthesis,
            ..
        }) = self.peek()
        {
            self.expect(TokenKind::OpenParenthesis)?;
            let _index = self.expect_str();
            self.expect(TokenKind::CloseParenthesis)?;
        }

        self.expect(TokenKind::Use)?;
        let _e = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::environment::Environment;

    fn do_check<F>(s: &str, check: F)
       where F: FnOnce(crate::errors::Result<crate::scanner::RawGPR>)
    {
        let mut env = Environment::default();
        let mut lex = crate::lexer::Lexer::new_from_string(s);
        let scan = crate::scanner::Scanner::new(&mut lex);
        check(scan.parse(&mut env.scenarios))
    }

    fn expect_error(s: &str, msg: &str) {
        do_check(
            s,
            |g| {
                match g {
                    Err(e) => assert_eq!(e.msg, msg),
                    Ok(_) => assert!(g.is_err(), "while parsing {}", s),
                }
            }
        )
    }

    fn expect_success<F>(s: &str, check: F)
       where F: FnOnce(&crate::scanner::RawGPR)
    {
        do_check(
            s,
            |g| {
                match &g {
                    Err(e) => assert!(
                        g.is_ok(), "while parsing {}, got error {}", s, e.msg),
                    Ok(g) => check(g),
                }
            }
        )
    }

    #[test]
    fn parse_errors() {
        expect_error(
            "project A is",
            "Unexpected end of file",
        );
    }

    #[test]
    fn parse_external() {
        expect_success(
            "project A is
                type Mode_Type is (\"debug\", \"optimize\", \"lto\");
                Mode : Mode_Type := external (\"MODE\");
            end A;",
            |g| {
                assert_eq!(
                    g.types.keys().collect::<Vec<&&str>>(),
                    vec![&"Mode_Type"]
                );
            },
        );
    }
}
