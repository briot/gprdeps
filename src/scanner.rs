use crate::lexer::{Lexer, Token};

type Result<R> = std::result::Result<R, String>;
type ParserResult = Result<()>;


pub struct Scanner {
}

impl Scanner {

    pub fn new() -> Self {
        Self {
        }
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    fn expect(&mut self, lex: &mut Lexer, token: Token) -> ParserResult {
        let tk = lex.next_token()?;
        if tk != token {
            return Err(format!("Expected {}, got {}", token, tk));
        }
        Ok(())
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str<'a> (&mut self, lex: &mut Lexer<'a>) -> Result<&'a [u8]> {
        match lex.next_token()? {
            Token::String(s) => Ok(s),
            t => Err(format!("Expected STRING, got {}", t))?,
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier<'a> (&mut self, lex: &mut Lexer<'a>) -> Result<&'a [u8]> {
        match lex.next_token()? {
            Token::Identifier(s) => Ok(s),
            t => Err(format!("Expected IDENTIFIER, got {}", t))?,
        }
    }

    fn expect_variable_reference<'a>(&mut self, lex: &mut Lexer<'a>) -> Result<&'a [u8]> {
        let mut varname = String::new();
        loop {
            match lex.next_token()? {
                Token::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
                tok  => Err(format!("Unexpected token {}", tok))?,
            }
            match lex.peek() {
                Token::Dot => { 
                    let _ = lex.next_token();
                    varname.push('.');
                },
                _          => break,
            }
        }
        Ok(b"")
    }

    fn expect_attribute_reference(&mut self, lex: &mut Lexer) -> Result<String> {
        let mut varname = String::new();
        match lex.next_token()? {
            Token::Identifier(s) => varname.push_str(std::str::from_utf8(s).unwrap()),
            tok  => Err(format!("Unexpected token {}", tok))?,
        }
        if *lex.peek() == Token::Dot {
            let _ = lex.next_token();
            let attname = self.expect_identifier(lex)?;
            varname.push('.');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }

        match lex.peek() {
            Token::Tick => {  // An attribute reference
                let _ = lex.next_token();
                let attname = self.expect_identifier(lex)?;
                varname.push('\'');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            Token::Dot => {   //  A variable in a package
                let _ = lex.next_token();
                let attname = self.expect_identifier(lex)?;
                varname.push('.');
                varname.push_str(std::str::from_utf8(attname).unwrap());
            },
            _ => {},
        }
        if *lex.peek() == Token::OpenParenthesis {
            self.expect(lex, Token::OpenParenthesis)?;
            let index = self.expect_str(lex)?;
            varname.push('(');
            varname.push_str(std::str::from_utf8(index).unwrap());
            varname.push(')');
            self.expect(lex, Token::CloseParenthesis)?;
        }

        Ok(varname)
    }
    pub fn parse(&mut self, lex: &mut Lexer) -> ParserResult {
        self.parse_file(lex)
    }

    /// Parse a whole file
    fn parse_file(&mut self, lex: &mut Lexer) -> ParserResult {
        loop {
            match lex.peek() {
                Token::EOF     => return Ok(()),
                Token::Project => self.parse_project_declaration(lex)?,
                Token::With    => self.parse_with_clause(lex)?,
                t              => Err(format!("Unexpected {}", t).to_string())?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::With)?;
        let _path = self.expect_str(lex)?;
        self.expect(lex, Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_declaration(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::Project)?;
        let name = self.expect_identifier(lex)?;

        if *lex.peek() == Token::Extends {
            self.parse_project_extension(lex)?;
        }

        self.expect(lex, Token::Is)?;

        loop {
            match lex.peek() {
                Token::End => break,
                Token::For => self.parse_attribute_declaration(lex)?,
                Token::Null => {},
                Token::Case => self.parse_case_statement(lex)?,
                Token::Package => self.parse_package_declaration(lex)?,
                tok  => Err(format!("Unexpected token {}", tok))?,
            }
        }

        self.expect(lex, Token::End)?;
        let endname = self.expect_identifier(lex)?;
        if name != endname {
            return Err(format!("Expected endname {:?}, got {:?}", name, endname));
        }
        self.expect(lex, Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::Extends)?;
        let _extended = self.expect_str(lex)?;
        Ok(())
    }

    fn parse_package_declaration(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::Package)?;
        let name = self.expect_identifier(lex)?;

        if *lex.peek() == Token::Extends {
            self.expect(lex, Token::Extends)?;
            let _extended = self.expect_variable_reference(lex)?;
        }

        self.expect(lex, Token::Is)?;

        loop {
            match lex.peek() {
                Token::End => break,
                Token::For => self.parse_attribute_declaration(lex)?,
                Token::Null => {},
                Token::Case => self.parse_case_statement(lex)?,
                tok  => Err(format!("Unexpected token {}", tok))?,
            }
        }

        self.expect(lex, Token::End)?;
        let endname = self.expect_identifier(lex)?;
        if name != endname {
            return Err(format!("Expected endname {:?}, got {:?}", name, endname));
        }
        self.expect(lex, Token::Semicolon)?;

        Ok(())
    }

    fn parse_case_statement(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::Case)?;
        let _varname = self.expect_variable_reference(lex);
        self.expect(lex, Token::Is)?;

        loop {
            match lex.next_token()? {
                Token::End => {
                    self.expect(lex, Token::Case)?;
                    self.expect(lex, Token::Semicolon)?;
                    break;
                },
                Token::When => {
                    loop {
                        match lex.next_token()? {
                            Token::String(_s) => {},
                            tok  => Err(format!("Unexpected token {}", tok))?,
                        }
                        match lex.peek() {
                            Token::Pipe => {},
                            Token::Arrow => {
                                let _ = lex.next_token();
                                break;
                            }
                            tok  => Err(format!("Unexpected token {}", tok))?,
                        }
                    }

                    loop {
                        match lex.peek() {
                            Token::End | Token::When => break,
                            Token::For => self.parse_attribute_declaration(lex)?,
                            Token::Null => {
                                let _ = lex.next_token();
                                self.expect(lex, Token::Semicolon)?;
                            },
                            Token::Case => self.parse_case_statement(lex)?,
                            tok  => Err(format!("Unexpected token {}", tok))?,
                        }
                    }
                },
                tok  => Err(format!("Unexpected token {:?}", tok))?,
            }
        }

        Ok(())
    }

    fn parse_attribute_declaration(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::For)?;
        let _attname = self.expect_str(lex);

        if *lex.peek() == Token::OpenParenthesis {
            self.expect(lex, Token::OpenParenthesis)?;
            let _index = self.expect_str(lex);
            self.expect(lex, Token::CloseParenthesis)?;
        }

        self.expect(lex, Token::Use)?;

        loop {

            match lex.peek() {
                Token::String(_) => {
                    let _strval = self.expect_str(lex);
                },
                Token::OpenParenthesis => {
                    let _ = lex.next_token()?;
                    if *lex.peek() == Token::CloseParenthesis {
                        let _ = lex.next_token()?;
                        // Empty list
                    } else {
                        loop {
                            match lex.next_token()? {
                                Token::String(_s) => {},
                                tok  => Err(format!("Unexpected token {}", tok))?,
                           }
                            match lex.next_token()? {
                                Token::CloseParenthesis => break,
                                Token::Comma => {},
                                tok  => Err(format!("Unexpected token {}", tok))?,
                           }
                        }
                    }
                },
                Token::Identifier(_prj_or_pkg_or_att) => {
                    let _att = self.expect_attribute_reference(lex)?;
                },
                tok  => Err(format!("Unexpected token {}", tok))?,
            }

            if *lex.peek() != Token::Ampersand {
                break;
            }
            let _ = lex.next_token()?;   // consume "&"
        }

        self.expect(lex, Token::Semicolon)?;
        Ok(())
    }
}
