use crate::errors::Result;
use crate::lexer::Lexer;
use crate::tokens::Token;

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
    fn expect(&mut self, token: Token) -> ParserResult {
        let tk = self.lex.next_token();
        if tk != token {
            return Err(self.lex.error(format!("Expected {}, got {}", token, tk)));
        }
        Ok(())
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a [u8]> {
        match self.lex.next_token() {
            Token::String(s) => Ok(s),
            t                => Err(self.lex.error(format!("Expected STRING, got {}", t))),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a [u8]> {
        match self.lex.next_token() {
            Token::Identifier(s) => Ok(s),
            t                    => Err(self.lex.error(format!("Expected IDENTIFIER, got {}", t))),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<&'a [u8]> {
        let mut varname = String::new();
        loop {
            match self.lex.next_token() {
                Token::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
            match self.lex.peek() {
                Token::Dot => { 
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
        match self.lex.next_token() {
            Token::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
            tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
        }
        if *self.lex.peek() == Token::Dot {
            let _ = self.lex.next_token();
            let attname = self.expect_identifier()?;
            varname.push('.');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }

        match self.lex.peek() {
            Token::Tick => {  // An attribute reference
                let _ = self.lex.next_token();
                let attname = self.expect_identifier()?;
                varname.push('\'');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            Token::Dot => {   //  A variable in a package
                let _ = self.lex.next_token();
                let attname = self.expect_identifier()?;
                varname.push('.');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            _ => {},
        }
        if *self.lex.peek() == Token::OpenParenthesis {
            self.expect(Token::OpenParenthesis)?;
            let index = self.expect_str()?;
            varname.push('(');
            varname.push_str(std::str::from_utf8(index).unwrap());
            varname.push(')');
            self.expect(Token::CloseParenthesis)?;
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
                Token::EOF     => return Ok(()),
                Token::Project => self.parse_project_declaration()?,
                Token::With    => self.parse_with_clause()?,
                t              => Err(self.lex.error(format!("Unexpected {}", t).to_string()))?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self) -> ParserResult {
        self.expect(Token::With)?;
        let _path = self.expect_str()?;
        self.expect(Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_declaration(&mut self) -> ParserResult {
        self.expect(Token::Project)?;
        let name = self.expect_identifier()?;

        if *self.lex.peek() == Token::Extends {
            self.parse_project_extension()?;
        }

        self.expect(Token::Is)?;

        loop {
            match self.lex.peek() {
                Token::End => break,
                Token::For => self.parse_attribute_declaration()?,
                Token::Null => {},
                Token::Case => self.parse_case_statement()?,
                Token::Package => self.parse_package_declaration()?,
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
        }

        self.expect(Token::End)?;
        let endname = self.expect_identifier()?;
        if name != endname {
            return Err(self.lex.error(format!("Expected endname {:?}, got {:?}", name, endname)));
        }
        self.expect(Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self) -> ParserResult {
        self.expect(Token::Extends)?;
        let _extended = self.expect_str()?;
        Ok(())
    }

    fn parse_package_declaration(&mut self) -> ParserResult {
        self.expect(Token::Package)?;
        let name = self.expect_identifier()?;

        if *self.lex.peek() == Token::Extends {
            self.expect(Token::Extends)?;
            let _extended = self.expect_variable_reference()?;
        }

        self.expect(Token::Is)?;

        loop {
            match self.lex.peek() {
                Token::End => break,
                Token::For => self.parse_attribute_declaration()?,
                Token::Null => {},
                Token::Case => self.parse_case_statement()?,
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
        }

        self.expect(Token::End)?;
        let endname = self.expect_identifier()?;
        if name != endname {
            return Err(self.lex.error(format!("Expected endname {:?}, got {:?}", name, endname)));
        }
        self.expect(Token::Semicolon)?;

        Ok(())
    }

    fn parse_case_statement(&mut self) -> ParserResult {
        self.expect(Token::Case)?;
        let _varname = self.expect_variable_reference();
        self.expect(Token::Is)?;

        loop {
            match self.lex.next_token() {
                Token::End => {
                    self.expect(Token::Case)?;
                    self.expect(Token::Semicolon)?;
                    break;
                },
                Token::When => {
                    loop {
                        match self.lex.next_token() {
                            Token::String(_s) => {},
                            tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                        }
                        match self.lex.peek() {
                            Token::Pipe => {},
                            Token::Arrow => {
                                let _ = self.lex.next_token();
                                break;
                            }
                            tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                        }
                    }

                    loop {
                        match self.lex.peek() {
                            Token::End | Token::When => break,
                            Token::For => self.parse_attribute_declaration()?,
                            Token::Null => {
                                let _ = self.lex.next_token();
                                self.expect(Token::Semicolon)?;
                            },
                            Token::Case => self.parse_case_statement()?,
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
        self.expect(Token::For)?;
        let _attname = self.expect_str();

        if *self.lex.peek() == Token::OpenParenthesis {
            self.expect(Token::OpenParenthesis)?;
            let _index = self.expect_str();
            self.expect(Token::CloseParenthesis)?;
        }

        self.expect(Token::Use)?;

        loop {

            match self.lex.peek() {
                Token::String(_) => {
                    let _strval = self.expect_str();
                },
                Token::OpenParenthesis => {
                    let _ = self.lex.next_token();
                    if *self.lex.peek() == Token::CloseParenthesis {
                        let _ = self.lex.next_token();
                        // Empty list
                    } else {
                        loop {
                            match self.lex.next_token() {
                                Token::String(_s) => {},
                                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                           }
                            match self.lex.next_token() {
                                Token::CloseParenthesis => break,
                                Token::Comma => {},
                                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                           }
                        }
                    }
                },
                Token::Identifier(_prj_or_pkg_or_att) => {
                    let _att = self.expect_attribute_reference()?;
                },
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }

            if *self.lex.peek() != Token::Ampersand {
                break;
            }
            let _ = self.lex.next_token();   // consume "&"
        }

        self.expect(Token::Semicolon)?;
        Ok(())
    }
}
