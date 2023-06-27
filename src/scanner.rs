use crate::errors::Result;
use crate::lexer::Lexer;
use crate::tokens::{Token, TokenKind};

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
    fn expect(&mut self, token: TokenKind) -> Result<()> {
        match self.lex.next() {
            Some(tk) if tk.kind == token => Ok(()),
            Some(tk) => Err(self.lex.error(format!("Expected {}, got {}", token, tk))),
            None => Err(self.lex.error("Unexpected end of file".into())),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => Err(self.lex.error("Unexpected end of file".into())),
            Some(Token {kind: TokenKind::String(s), .. }) => Ok(s),
            Some(t) => Err(self.lex.error(format!("Expected String, got {}", t))),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier(&mut self) -> Result<&'a [u8]> {
        match self.lex.next() {
            None => Err(self.lex.error("Unexpected end of file".into())),
            Some(Token {kind: TokenKind::Identifier(s), .. }) => Ok(s),
            Some(t) => Err(self.lex.error(format!("Expected Identifier, got {}", t))),
        }
    }

    fn expect_variable_reference(&mut self) -> Result<String> {
        let mut varname = String::new();
        loop {
            match self.lex.next() {
                None => Err(self.lex.error("Unexpected end of file".into()))?,
                Some(Token {kind: TokenKind::Project, .. }) => {
                    // e.g.  for source_dirs use project'source_dirs & ..
                    varname.push_str("project");
                },
                Some(Token {kind: TokenKind::Identifier(s), .. }) =>
                    varname.push_str(std::str::from_utf8(s).unwrap()),
                Some(t) => Err(self.lex.error(format!("Unexpected token {}", t)))?,
            }

            match self.lex.peek() {
                TokenKind::Dot => { 
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

        if *self.lex.peek() == TokenKind::Tick {  // An attribute reference
            let _ = self.lex.next();
            let attname = self.expect_identifier()?;
            varname.push('\'');
            varname.push_str(std::str::from_utf8(attname).unwrap());
        }
        if *self.lex.peek() == TokenKind::OpenParenthesis {
            self.parse_arg_list()?;
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
                TokenKind::EOF  => return Ok(()),
                TokenKind::With => self.parse_with_clause()?,
                _               => self.parse_project_declaration()?,
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
            let _ = self.lex.next();  // consume "aggregate"
        }

        if *self.lex.peek() == TokenKind::Library {
            let _ = self.lex.next();  // consume "library"
        }

        if *self.lex.peek() == TokenKind::Abstract {
            let _ = self.lex.next();  // consume "abstract"
        }

        self.expect(TokenKind::Project)?;

        let name = std::str::from_utf8(self.expect_identifier()?).unwrap();

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
                TokenKind::Identifier(_) => self.parse_variable_definition()?,
                TokenKind::Type => self.parse_type_definition()?,
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }
        }

        self.expect(TokenKind::End)?;
        let endname = std::str::from_utf8(self.expect_identifier()?).unwrap();
        if name.to_lowercase() != endname.to_lowercase() {
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

        if *self.lex.peek() == TokenKind::Extends {
            self.expect(TokenKind::Extends)?;
            let _extended = self.expect_variable_reference()?;
        }

        match self.lex.next() {
            None => Err(self.lex.error("Unexpected end of file".into()))?,
            Some(Token {kind: TokenKind::Is, .. }) => {
                loop {
                    match self.lex.peek() {
                        TokenKind::End => break,
                        TokenKind::For => self.parse_attribute_declaration()?,
                        TokenKind::Null => {},
                        TokenKind::Case => self.parse_case_statement()?,
                        TokenKind::Identifier(_) => self.parse_variable_definition()?,
                        tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                    }
                }

                self.expect(TokenKind::End)?;
                let endname = self.expect_identifier()?;
                if name != endname {
                    return Err(self.lex.error(
                        format!("Expected endname {:?}, got {:?}", name, endname)
                    ));
                }
            },
            Some(Token {kind: TokenKind::Renames, .. }) => {
                let _orig = self.expect_variable_reference();
            },
            Some(t) => Err(self.lex.error(format!("Unexpected {}", t)))?,
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(())
    }

    fn parse_variable_definition(&mut self) -> ParserResult {
        let _name = self.expect_identifier()?;

        if *self.lex.peek() == TokenKind::Colon {
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
                None => Err(self.lex.error("Unexpected end of file".into()))?,
                Some(Token {kind: TokenKind::End, .. }) => {
                    self.expect(TokenKind::Case)?;
                    self.expect(TokenKind::Semicolon)?;
                    break;
                },
                Some(Token {kind: TokenKind::When, .. }) => {
                    loop {
                        match self.lex.next() {
                            None => Err(self.lex.error("Unexpected end of file".into()))?,
                            Some(Token {kind: TokenKind::String(_s), .. }) => {},
                            Some(Token {kind: TokenKind::Others, .. }) => {
                                self.expect(TokenKind::Arrow)?;
                                break;
                            },
                            Some(t) => Err(
                                self.lex.error(format!("Unexpected token {} in when", t)))?,
                        }
                        match self.lex.next() {
                            None => Err(self.lex.error("Unexpected end of file".into()))?,
                            Some(Token {kind: TokenKind::Pipe, .. }) => {},
                            Some(Token {kind: TokenKind::Arrow, .. }) => break,
                            Some(t) => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                        }
                    }

                    loop {
                        match self.lex.peek() {
                            TokenKind::EOF => Err(self.lex.error("Unexpected end of file".into()))?,
                            TokenKind::End | TokenKind::When => break,
                            TokenKind::For => self.parse_attribute_declaration()?,
                            TokenKind::Null => {
                                let _ = self.lex.next();
                                self.expect(TokenKind::Semicolon)?;
                            },
                            TokenKind::Case => self.parse_case_statement()?,
                            TokenKind::Identifier(_) => self.parse_variable_definition()?,
                            tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
                        }
                    }
                },
                Some(t) => Err(self.lex.error(format!("Unexpected token {}", t)))?,
            }
        }

        Ok(())
    }

    fn parse_arg_list(&mut self) -> ParserResult {
        self.expect(TokenKind::OpenParenthesis)?;
        if *self.lex.peek() == TokenKind::Others {
            let _ = self.lex.next();  // consume "others"
        } else {
            loop {
                match self.lex.peek() {
                    TokenKind::String(_) => {
                        let _args = self.parse_string_expression();
                    },
                    TokenKind::Identifier(_) => {
                        let _id = self.expect_attribute_reference()?;
                        if *self.lex.peek() == TokenKind::OpenParenthesis {
                            self.parse_arg_list()?;
                        }
                    },
                    tok  => Err(self.lex.error(format!("Unexpected token {:?}", tok)))?,
                }

                if *self.lex.peek() != TokenKind::Comma {
                    break;
                }
                let _ = self.lex.next();  // consume ','
            }
        }
        self.expect(TokenKind::CloseParenthesis)?;
        Ok(())
    }

    fn parse_string_expression(&mut self) -> ParserResult {
        loop {
            match self.lex.peek() {
                TokenKind::String(_) => {
                    let _s = self.lex.next();  //  consume the string
                },
                TokenKind::Identifier(_) => {
                    // e.g.  for object_dir use "../" & shared'object_dir
                    let _s = self.expect_attribute_reference()?;
                },
                TokenKind::EOF => Err(self.lex.error("Unexpected end of file".into()))?,
                t    => {
                    Err(self.lex.error(format!("Unexpected token in string expression {}", t)))?;
                },
            }

            match self.lex.peek() {
                TokenKind::Ampersand  => {
                    let _ = self.lex.next();   // consume "&"
                },
                _ => {
                    break;
                },
            }
        }
        Ok(())
    }

    fn parse_expression(&mut self) -> ParserResult {
        loop {
            match self.lex.peek() {
                TokenKind::String(_) => {
                    self.parse_string_expression()?;
                },
                TokenKind::OpenParenthesis => {
                    let _ = self.lex.next();
                    if *self.lex.peek() == TokenKind::CloseParenthesis {
                        let _ = self.lex.next();   //  consume closing parenthesis
                        // Empty list
                    } else {
                        loop {
                            self.parse_string_expression()?;
                            match self.lex.next() {
                                None => Err(self.lex.error("Unexpected end of file".into()))?,
                                Some(Token {kind: TokenKind::CloseParenthesis, .. }) => break,
                                Some(Token {kind: TokenKind::Comma, .. }) => {},
                                Some(t) => Err(self.lex.error(format!("Unexpected token {}", t)))?,
                           }
                        }
                    }
                },
                TokenKind::Identifier(_) | TokenKind::Project => {
                    let _att = self.expect_attribute_reference()?;
                },
                tok  => Err(self.lex.error(format!("Unexpected token {}", tok)))?,
            }

            if *self.lex.peek() != TokenKind::Ampersand {
                break;
            }
            let _ = self.lex.next();   // consume "&"
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
        self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(())
    }
}
