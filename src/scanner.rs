use crate::errors::Result;
use crate::lexer::Lexer;
use crate::tokens::{Token, TokenKind};
use crate::gpr::{Abstract, Aggregate, Environment, Library, GPR, RawGPR};

type ParserResult = Result<()>;

pub struct Scanner<'a> {
    lex: &'a mut Lexer<'a>,
    gpr: RawGPR<'a>,
}

impl<'a> Scanner<'a> {

    pub fn new(lex: &'a mut Lexer<'a>) -> Self {
        Self {
            gpr: RawGPR::default(),
            lex,
        }
    }

    pub fn parse(mut self, env: &Environment) -> Result<GPR> {
        self.parse_file()?;
        self.gpr.path = self.lex.path().to_path_buf();
        Ok(GPR::new(env, self.gpr))
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
            None     => self.error("Unexpected end of file".into()),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {kind: TokenKind::String(s), .. }) => Ok(s),
            Some(t) => self.error(format!("Expected String, got {}", t)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => self.error("Unexpected end of file".into()),
            Some(Token {kind: TokenKind::Identifier(s), .. }) => Ok(s),
            Some(t) => self.error(format!("Expected Identifier, got {}", t)),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<String> {
        let mut varname = String::new();
        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {kind: TokenKind::Project, .. }) => {
                    // e.g.  for source_dirs use project'source_dirs & ..
                    varname.push_str("project");
                },
                Some(Token {kind: TokenKind::Identifier(s), .. }) =>
                    varname.push_str(std::str::from_utf8(s).unwrap()),
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }

            match self.peek() {
                Some(Token {kind: TokenKind::Dot, .. }) => { 
                    let _ = self.lex.next();
                    varname.push('.');
                },
                _ => break,
            }
        }
        Ok(varname)
    }

    fn expect_attribute_reference(&mut self) -> Result<String> {
        let mut varname = self.expect_variable_reference()?;

        if let Some(Token {kind: TokenKind::Tick, .. }) = self.peek() {   // attribute ref
            let _ = self.lex.next();
            let attname = self.expect_identifier()?;
            varname.push('\'');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }
        if let Some(Token {kind: TokenKind::OpenParenthesis, .. }) = self.peek() {
            self.parse_arg_list()?;
        }

        Ok(varname)
    }

    /// Parse a whole file
    fn parse_file(&mut self) -> ParserResult {
        loop {
            match self.peek() {
                None                                     => return Ok(()),
                Some(Token {kind: TokenKind::With, .. }) => self.parse_with_clause()?,
                _                                        => self.parse_project_declaration()?,
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

    fn parse_project_declaration(&mut self) -> ParserResult {
        if let Some(Token {kind: TokenKind::Aggregate, .. }) = self.peek() {
            let _ = self.lex.next();  // consume "aggregate"
            self.gpr.is_aggregate = Aggregate::IsAggregate;
        }

        if let Some(Token {kind: TokenKind::Library, .. }) = self.peek() {
            let _ = self.lex.next();  // consume "library"
            self.gpr.is_library = Library::IsLibrary;
        }

        if let Some(Token {kind: TokenKind::Abstract, .. }) = self.peek() {
            let _ = self.lex.next();  // consume "abstract"
            self.gpr.is_abstract = Abstract::IsAbstract;
        }

        self.expect(TokenKind::Project)?;

        let name = std::str::from_utf8(self.expect_identifier()?).unwrap();
        self.gpr.name = name;

        if let Some(Token {kind: TokenKind::Extends, .. }) = self.peek() {
            self.parse_project_extension()?;
        }

        self.expect(TokenKind::Is)?;

        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {kind: TokenKind::End, ..}) => break,
                Some(Token {kind: TokenKind::For, ..}) => self.parse_attribute_declaration()?,
                Some(Token {kind: TokenKind::Null, ..}) => {},
                Some(Token {kind: TokenKind::Case, ..}) => self.parse_case_statement()?,
                Some(Token {kind: TokenKind::Package, ..}) => self.parse_package_declaration()?,
                Some(Token {kind: TokenKind::Identifier(_), ..}) => self.parse_variable_definition()?,
                Some(Token {kind: TokenKind::Type, ..}) => self.parse_type_definition()?,
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

    fn parse_project_extension(&mut self) -> ParserResult {
        self.expect(TokenKind::Extends)?;
        let _extended = self.expect_str()?;
        Ok(())
    }

    fn parse_type_definition(&mut self) -> ParserResult {
        self.expect(TokenKind::Type)?;
        let _name = self.expect_identifier()?;
        self.expect(TokenKind::Is)?;
        self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_package_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::Package)?;
        let name = self.expect_identifier()?;

        if let Some(Token {kind: TokenKind::Extends, .. }) = self.peek() {
            self.expect(TokenKind::Extends)?;
            let _extended = self.expect_variable_reference()?;
        }

        match self.lex.next() {
            None => self.error("Unexpected end of file".into())?,
            Some(Token {kind: TokenKind::Is, .. }) => {
                loop {
                    match self.peek() {
                        None => self.error("Unexpected end of file".into())?,
                        Some(Token {kind: TokenKind::End, ..}) => break,
                        Some(Token {kind: TokenKind::For, ..}) => self.parse_attribute_declaration()?,
                        Some(Token {kind: TokenKind::Null, ..}) => {},
                        Some(Token {kind: TokenKind::Case, ..}) => self.parse_case_statement()?,
                        Some(Token {kind: TokenKind::Identifier(_), ..}) =>
                            self.parse_variable_definition()?,
                        Some(t)  => self.error(format!("Unexpected token {}", t))?,
                    }
                }

                self.expect(TokenKind::End)?;
                let endname = self.expect_identifier()?;
                if name != endname {
                    return self.error(format!("Expected endname {:?}, got {:?}", name, endname));
                }
            },
            Some(Token {kind: TokenKind::Renames, .. }) => {
                let _orig = self.expect_variable_reference();
            },
            Some(t) => self.error(format!("Unexpected {}", t))?,
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(())
    }

    fn parse_variable_definition(&mut self) -> ParserResult {
        let _name = self.expect_identifier()?;

        if let Some(Token {kind: TokenKind::Colon, .. }) = self.peek() {
            let _ = self.lex.next();  // consume ":"
            let _type = self.expect_variable_reference()?;  // Could be qualified name
        }

        self.expect(TokenKind::Assign)?;
        self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_case_statement(&mut self) -> ParserResult {
        self.expect(TokenKind::Case)?;
        let _varname = self.expect_variable_reference();
        self.expect(TokenKind::Is)?;

        loop {
            match self.lex.next() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {kind: TokenKind::End, .. }) => {
                    self.expect(TokenKind::Case)?;
                    self.expect(TokenKind::Semicolon)?;
                    break;
                },
                Some(Token {kind: TokenKind::When, .. }) => {
                    loop {
                        match self.lex.next() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {kind: TokenKind::String(_s), .. }) => {},
                            Some(Token {kind: TokenKind::Others, .. }) => {
                                self.expect(TokenKind::Arrow)?;
                                break;
                            },
                            Some(t) => self.error(format!("Unexpected token {} in when", t))?,
                        }
                        match self.lex.next() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {kind: TokenKind::Pipe, .. }) => {},
                            Some(Token {kind: TokenKind::Arrow, .. }) => break,
                            Some(t) => self.error(format!("Unexpected token {}", t))?,
                        }
                    }

                    loop {
                        match self.peek() {
                            None => self.error("Unexpected end of file".into())?,
                            Some(Token {kind: TokenKind::End | TokenKind::When, ..}) => break,
                            Some(Token {kind: TokenKind::For, ..}) =>
                                self.parse_attribute_declaration()?,
                            Some(Token {kind: TokenKind::Null, ..}) => {
                                let _ = self.lex.next();
                                self.expect(TokenKind::Semicolon)?;
                            },
                            Some(Token {kind: TokenKind::Case, ..}) =>
                                self.parse_case_statement()?,
                            Some(Token {kind: TokenKind::Identifier(_), ..}) =>
                                self.parse_variable_definition()?,
                            Some(t)  => self.error(format!("Unexpected token {}", t))?,
                        }
                    }
                },
                Some(t) => self.error(format!("Unexpected token {}", t))?,
            }
        }

        Ok(())
    }

    fn parse_arg_list(&mut self) -> ParserResult {
        self.expect(TokenKind::OpenParenthesis)?;

        if let Some(Token {kind: TokenKind::Others, .. }) = self.peek() {
            let _ = self.lex.next();  // consume "others"
        } else {
            loop {
                match self.peek() {
                    None => self.error("Unexpected end of file".into())?,
                    Some(Token {kind: TokenKind::String(_), ..}) => {
                        let _args = self.parse_string_expression();
                    },
                    Some(Token {kind: TokenKind::Identifier(_), ..}) => {
                        let _id = self.expect_attribute_reference()?;
                        if let Some(Token {kind: TokenKind::OpenParenthesis, .. }) = self.peek() {
                            self.parse_arg_list()?;
                        }
                    },
                    Some(t) => self.error(format!("Unexpected token {}", t))?,
                }

                if let Some(Token {kind: TokenKind::Comma, .. }) = self.peek() {
                    let _ = self.lex.next();  // consume ','
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::CloseParenthesis)?;
        Ok(())
    }

    fn parse_string_expression(&mut self) -> ParserResult {
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {kind: TokenKind::String(_), ..}) => {
                    let _s = self.lex.next();  //  consume the string
                },
                Some(Token {kind: TokenKind::Identifier(_), ..}) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let _s = self.expect_attribute_reference()?;
                },
                Some(t)    => self.error(format!("Unexpected token in string expression {}", t))?,
            }

            if let Some(Token {kind: TokenKind::Ampersand, .. }) = self.peek() {
                let _ = self.lex.next();   // consume "&"
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_expression(&mut self) -> ParserResult {
        loop {
            match self.peek() {
                None => self.error("Unexpected end of file".into())?,
                Some(Token {kind: TokenKind::String(_), ..}) => {
                    self.parse_string_expression()?;
                },
                Some(Token {kind: TokenKind::OpenParenthesis, ..}) => {
                    let _ = self.lex.next();   // consume "("
                    if let Some(Token {kind: TokenKind::CloseParenthesis, .. }) = self.peek() {
                        let _ = self.lex.next();   //  consume ")"
                        // Empty list
                    } else {
                        loop {
                            self.parse_string_expression()?;
                            match self.lex.next() {
                                None => self.error("Unexpected end of file".into())?,
                                Some(Token {kind: TokenKind::CloseParenthesis, .. }) => break,
                                Some(Token {kind: TokenKind::Comma, .. }) => {},
                                Some(t) => self.error(format!("Unexpected token {}", t))?,
                           }
                        }
                    }
                },
                Some(Token {kind: TokenKind::Identifier(_) | TokenKind::Project, ..}) => {
                    let _att = self.expect_attribute_reference()?;
                },
                Some(t)  => self.error(format!("Unexpected token {}", t))?,
            }

            if let Some(Token {kind: TokenKind::Ampersand, .. }) = self.peek() {
                let _ = self.lex.next();   // consume "&"
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_attribute_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::For)?;
        let _attname = self.expect_str();

        if let Some(Token {kind: TokenKind::OpenParenthesis, .. }) = self.peek() {
            self.expect(TokenKind::OpenParenthesis)?;
            let _index = self.expect_str();
            self.expect(TokenKind::CloseParenthesis)?;
        }

        self.expect(TokenKind::Use)?;
        self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }
}
