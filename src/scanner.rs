use crate::errors::Result;
use crate::lexer::Lexer;
use crate::tokens::TokenKind;

type ParserResult = Result<()>;


pub struct Scanner<'a> {
    lex: &'a mut Lexer<'a>,
}

impl<'a> Scanner<'a> {

    pub fn new(lex: &'a mut Lexer<'a>) -> Self {
        Self {
            lex,
        }
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    fn expect(&mut self, token: TokenKind) -> ParserResult {
        let tk = self.lex.next_token();
        if tk.kind != token {
            return Err(self.lex.error(format!("Expected {}, got {}", token, tk)));
        }
        Ok(())
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a [u8]> {
        let t = self.lex.next_token();
        match t.kind {
            TokenKind::String(s) => Ok(s),
            _                    => Err(self.lex.error(format!("Expected STRING, got {}", t))),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a [u8]> {
        let t = self.lex.next_token();
        match t.kind {
            TokenKind::Identifier(s) => Ok(s),
            _                    => Err(self.lex.error(format!("Expected IDENTIFIER, got {}", t))),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<&'a [u8]> {
        let mut varname = String::new();
        loop {
            let t = self.lex.next_token();
            match t.kind {
                TokenKind::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
                _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
            }
            match self.lex.peek() {
                TokenKind::Dot => { 
                    let _ = self.lex.next_token();
                    varname.push('.');
                },
                _          => break,
            }
        }
        Ok(b"")
    }

    fn expect_attribute_reference(&mut self) -> Result<String> {
        let mut varname = String::new();
        let t = self.lex.next_token();
        match t.kind {
            TokenKind::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
            _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
        }
        if *self.lex.peek() == TokenKind::Dot {
            let _ = self.lex.next_token();
            let attname = self.expect_identifier()?;
            varname.push('.');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }

        match self.lex.peek() {
            TokenKind::Tick => {  // An attribute reference
                let _ = self.lex.next_token();
                let attname = self.expect_identifier()?;
                varname.push('\'');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            TokenKind::Dot => {   //  A variable in a package
                let _ = self.lex.next_token();
                let attname = self.expect_identifier()?;
                varname.push('.');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            _ => {},
        }
        if *self.lex.peek() == TokenKind::OpenParenthesis {
            self.expect(TokenKind::OpenParenthesis)?;
            let index = self.expect_str()?;
            varname.push('(');
            varname.push_str(std::str::from_utf8(index).unwrap());
            varname.push(')');
            self.expect(TokenKind::CloseParenthesis)?;
        }

        Ok(varname)
    }
    pub fn parse(&mut self) -> ParserResult {
        self.parse_file()
    }

    /// Parse a whole file
    fn parse_file(&mut self) -> ParserResult {
        loop {
            match self.lex.peek() {
                TokenKind::EOF     => return Ok(()),
                TokenKind::With    => self.parse_with_clause()?,
                _              => self.parse_project_declaration()?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self) -> ParserResult {
        self.expect(TokenKind::With)?;
        let _path = self.expect_str()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_declaration(&mut self) -> ParserResult {
        if *self.lex.peek() == TokenKind::Aggregate {
            let _ = self.lex.next_token();  // consume "aggregate"
        }

        if *self.lex.peek() == TokenKind::Library {
            let _ = self.lex.next_token();  // consume "library"
        }

        self.expect(TokenKind::Project)?;

        let name = self.expect_identifier()?;

        if *self.lex.peek() == TokenKind::Extends {
            self.parse_project_extension()?;
        }

        self.expect(TokenKind::Is)?;

        loop {
            match self.lex.peek() {
                TokenKind::End => break,
                TokenKind::For => self.parse_attribute_declaration()?,
                TokenKind::Null => {},
                TokenKind::Case => self.parse_case_statement()?,
                TokenKind::Package => self.parse_package_declaration()?,
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
        }

        self.expect(TokenKind::End)?;
        let endname = self.expect_identifier()?;
        if name != endname {
            return Err(self.lex.error(format!("Expected endname {:?}, got {:?}", name, endname)));
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> ParserResult {
        self.expect(TokenKind::Extends)?;
        let _extended = self.expect_str()?;
        Ok(())
    }

    fn parse_package_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::Package)?;
        let name = self.expect_identifier()?;

        if *self.lex.peek() == TokenKind::Extends {
            self.expect(TokenKind::Extends)?;
            let _extended = self.expect_variable_reference()?;
        }

        self.expect(TokenKind::Is)?;

        loop {
            match self.lex.peek() {
                TokenKind::End => break,
                TokenKind::For => self.parse_attribute_declaration()?,
                TokenKind::Null => {},
                TokenKind::Case => self.parse_case_statement()?,
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
        }

        self.expect(TokenKind::End)?;
        let endname = self.expect_identifier()?;
        if name != endname {
            return Err(self.lex.error(format!("Expected endname {:?}, got {:?}", name, endname)));
        }
        self.expect(TokenKind::Semicolon)?;

        Ok(())
    }

    fn parse_case_statement(&mut self) -> ParserResult {
        self.expect(TokenKind::Case)?;
        let _varname = self.expect_variable_reference();
        self.expect(TokenKind::Is)?;

        loop {
            let t = self.lex.next_token();
            match t.kind {
                TokenKind::End => {
                    self.expect(TokenKind::Case)?;
                    self.expect(TokenKind::Semicolon)?;
                    break;
                },
                TokenKind::When => {
                    loop {
                        let t = self.lex.next_token();
                        match t.kind {
                            TokenKind::String(_s) => {},
                            _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                        }
                        let t = self.lex.next_token();
                        match t.kind {
                            TokenKind::Pipe => {},
                            TokenKind::Arrow => break,
                            _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                        }
                    }

                    loop {
                        match self.lex.peek() {
                            TokenKind::End | TokenKind::When => break,
                            TokenKind::For => self.parse_attribute_declaration()?,
                            TokenKind::Null => {
                                let _ = self.lex.next_token();
                                self.expect(TokenKind::Semicolon)?;
                            },
                            TokenKind::Case => self.parse_case_statement()?,
                            tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                        }
                    }
                },
                tok  => Err(self.lex.error(format!("Unexpected token {:?}", tok)))?,
            }
        }

        Ok(())
    }

    fn parse_attribute_declaration(&mut self) -> ParserResult {
        self.expect(TokenKind::For)?;
        let _attname = self.expect_str();

        if *self.lex.peek() == TokenKind::OpenParenthesis {
            self.expect(TokenKind::OpenParenthesis)?;
            let _index = self.expect_str();
            self.expect(TokenKind::CloseParenthesis)?;
        }

        self.expect(TokenKind::Use)?;

        loop {

            match self.lex.peek() {
                TokenKind::String(_) => {
                    let _strval = self.expect_str();
                },
                TokenKind::OpenParenthesis => {
                    let _ = self.lex.next_token();
                    if *self.lex.peek() == TokenKind::CloseParenthesis {
                        let _ = self.lex.next_token();
                        // Empty list
                    } else {
                        loop {
                            let t = self.lex.next_token();
                            match t.kind {
                                TokenKind::String(_s) => {},
                                _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                            }
                            let t = self.lex.next_token();
                            match t.kind {
                                TokenKind::CloseParenthesis => break,
                                TokenKind::Comma => {},
                                _    => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                           }
                        }
                    }
                },
                TokenKind::Identifier(_prj_or_pkg_or_att) => {
                    let _att = self.expect_attribute_reference()?;
                },
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }

            if *self.lex.peek() != TokenKind::Ampersand {
                break;
            }
            let _ = self.lex.next_token();   // consume "&"
        }

        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }
}
