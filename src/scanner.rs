use crate::errors::Result;
use crate::lexer::Lexer;
use crate::files::File;
use crate::rawexpr::{
    AttributeDecl, AttributeName, CaseStmt, PackageDecl, RawExpr, Statement,
    StringOrOthers, TypeDecl, VariableDecl, VariableName, WhenClause, PROJECT,
};
use crate::rawgpr::RawGPR;
use crate::tokens::{Token, TokenKind};

pub struct Scanner<'a> {
    lex: Lexer<'a>,
    gpr: RawGPR<'a>,
}

impl<'a> Scanner<'a> {
    pub fn new(file: &'a File) -> Self {
        Self {
            gpr: RawGPR::new(file.path()),
            lex: Lexer::new(file),
        }
    }

    pub fn parse(&mut self) -> Result<()> {
        self.parse_file()?;
        Ok(())
    }

    #[inline]
    pub fn gpr(&self) -> &RawGPR<'a> {
        &self.gpr
    }

    #[inline]
    fn error<T>(&self, msg: String) -> Result<T> {
        Err(self.lex.error(msg))
    }

    #[inline]
    fn peek(&self) -> Option<Token> {
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
            }) => Ok(s),
            Some(t) => self.error(format!("Expected String, got {}", t)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or the keyword "others"
    fn expect_str_or_others(&mut self) -> Result<StringOrOthers<'a>> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::Others,
                ..
            }) => Ok(StringOrOthers::Others),
            Some(Token {
                kind: TokenKind::String(s),
                ..
            }) => Ok(StringOrOthers::Str(s)),
            Some(t) => {
                self.error(format!("Expected String or others, got {}", t))
            }
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a str> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {
                kind: TokenKind::Identifier(s),
                ..
            }) => Ok(s),
            Some(t) => self.error(format!("Expected Identifier, got {}", t)),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<VariableName<'a>> {
        let mut result = VariableName::default();
        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::Project,
                    ..
                }) => {
                    // e.g.  for source_dirs use project'source_dirs & ..
                    result.name = PROJECT;
                }
                Some(Token {
                    kind: TokenKind::Identifier(s),
                    ..
                }) => result.name = s,
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
                    result.name = "";
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn expect_attribute_reference(&mut self) -> Result<AttributeName<'a>> {
        let varname = self.expect_variable_reference()?;
        let mut qname = AttributeName {
            project: varname.project,
            package: varname.package,
            attname: varname.name,
            index: None,
        };

        if let Some(Token {
            kind: TokenKind::Tick,
            ..
        }) = self.peek()
        {
            // attribute ref
            let _ = self.lex.next();
            qname.attname = self.expect_identifier()?;
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
        self.gpr.imported.push(path);

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
            self.gpr.extends = Some(self.parse_project_extension()?);
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
                    self.parse_attribute_declaration()?,
                )),
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
                "Expected endname {}, got {:?}",
                self.gpr.name, endname
            ));
        }
        self.expect(TokenKind::Semicolon)?;

        self.gpr.body = body;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> Result<&'a str> {
        self.expect(TokenKind::Extends)?;
        self.expect_str()
    }

    fn parse_type_definition(&mut self) -> Result<Statement<'a>> {
        self.expect(TokenKind::Type)?;
        let typename = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Type(TypeDecl {
            typename,
            valid: expr.to_static_list(&self.lex)?,
        }))
    }

    fn parse_package_declaration(&mut self) -> Result<Statement<'a>> {
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
                        }) => result.body.push(Statement::Attribute(
                            self.parse_attribute_declaration()?,
                        )),
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
                        }) => {
                            result.body.push(self.parse_variable_definition()?)
                        }
                        Some(t) => {
                            self.error(format!("Unexpected token {}", t))?
                        }
                    }
                }

                self.expect(TokenKind::End)?;
                let endname = self.expect_identifier()?;
                if result.name != endname {
                    return self.error(format!(
                        "Expected endname {:?}, got {:?}",
                        result.name, endname
                    ));
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

    fn parse_variable_definition(&mut self) -> Result<Statement<'a>> {
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

    fn parse_case_statement(&mut self) -> Result<Statement<'a>> {
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
                            None => {
                                self.error("Unexpected end of file".into())?
                            }
                            Some(Token {
                                kind: TokenKind::String(s),
                                ..
                            }) => when.values.push(StringOrOthers::Str(s)),
                            Some(Token {
                                kind: TokenKind::Others,
                                ..
                            }) => {
                                self.expect(TokenKind::Arrow)?;
                                when.values.push(StringOrOthers::Others);
                                break;
                            }
                            Some(t) => self.error(format!(
                                "Unexpected token {} in when",
                                t
                            ))?,
                        }
                        match self.lex.next() {
                            None => {
                                self.error("Unexpected end of file".into())?
                            }
                            Some(Token {
                                kind: TokenKind::Pipe,
                                ..
                            }) => {}
                            Some(Token {
                                kind: TokenKind::Arrow,
                                ..
                            }) => break,
                            Some(t) => {
                                self.error(format!("Unexpected token {}", t))?
                            }
                        }
                    }

                    loop {
                        match self.peek() {
                            None => {
                                self.error("Unexpected end of file".into())?
                            }
                            Some(Token {
                                kind: TokenKind::End | TokenKind::When,
                                ..
                            }) => break,
                            Some(Token {
                                kind: TokenKind::For,
                                ..
                            }) => when.body.push(Statement::Attribute(
                                self.parse_attribute_declaration()?,
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
                            }) => when.body.push(self.parse_case_statement()?),
                            Some(Token {
                                kind: TokenKind::Identifier(_),
                                ..
                            }) => when
                                .body
                                .push(self.parse_variable_definition()?),
                            Some(t) => {
                                self.error(format!("Unexpected token {}", t))?
                            }
                        }
                    }

                    result.when.push(when);
                }
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }
        Ok(Statement::Case(result))
    }

    fn parse_arg_list(&mut self) -> Result<RawExpr<'a>> {
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
                        result = result.comma(RawExpr::AttributeOrFunc(s));
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
    fn parse_string_expression(&mut self) -> Result<RawExpr<'a>> {
        let mut result = RawExpr::Empty;
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {
                    kind: TokenKind::String(_),
                    ..
                }) => {
                    let s = self.expect_str()?;
                    result = result.ampersand(RawExpr::StaticString(s));
                },
                Some(Token {
                    kind: TokenKind::Identifier(_),
                    ..
                }) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let att = self.expect_attribute_reference()?;
                    result = result.ampersand(RawExpr::AttributeOrFunc(att));
                },
                Some(t) => self.error(format!(
                    "Unexpected token in string expression {}",
                    t
                ))?,
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

    fn parse_expression(&mut self) -> Result<RawExpr<'a>> {
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
                                None => {
                                    self.error("Unexpected end of file".into())?
                                }
                                Some(Token {
                                    kind: TokenKind::CloseParenthesis,
                                    ..
                                }) => break,
                                Some(Token {
                                    kind: TokenKind::Comma,
                                    ..
                                }) => {}
                                Some(t) => self
                                    .error(format!("Unexpected token {}", t))?,
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
                    result = result.ampersand(RawExpr::AttributeOrFunc(att));
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

    fn parse_attribute_declaration(&mut self) -> Result<AttributeDecl<'a>> {
        let mut result = AttributeDecl::default();

        self.expect(TokenKind::For)?;
        result.name = self.expect_identifier()?;

        if let Some(Token {
            kind: TokenKind::OpenParenthesis,
            ..
        }) = self.peek()
        {
            self.expect(TokenKind::OpenParenthesis)?;
            result.index = Some(self.expect_str_or_others()?);
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
        let file = crate::files::File::new_from_str(s);
        let mut scan = crate::scanner::Scanner::new(&file);
        match scan.parse() {
            Err(e) => check(Err(e)),
            Ok(_) => check(Ok(scan.gpr)),
        }
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
            Err(e) => {
                assert!(g.is_ok(), "while parsing {}, got error {}", s, e.msg)
            }
            Ok(g) => check(g),
        })
    }

    #[test]
    fn parse_errors() {
        expect_error("project A is", "Unexpected end of file");
    }

    #[test]
    fn parse_attribute_decl() {
        expect_success(
            "project A is
                for Source_Files use (\"a.adb\");
                package Linker is
                   for Switches (others) use ();
                end Linker;
             end A;",
            |_g| {},
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
                println!("MANU {}  {:?}", g.name, g.body);
                //                assert_eq!(g.types.keys().collect::<Vec<&&str>>(), vec![&"Mode_Type"]);
            },
        );
    }

    //    ... tests extends
}
