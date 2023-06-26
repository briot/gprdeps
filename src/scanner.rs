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
            return Err(format!("Expected {:?}, got {:?}", token, tk));
        }
        Ok(())
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    fn expect_str (&mut self, lex: &mut Lexer) -> Result<String> {
        match lex.next_token()? {
            Token::String(s) => Ok(s),
            t => Err(format!("Expected STRING, got {:?}", t))?,
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.
    fn expect_identifier (&mut self, lex: &mut Lexer) -> Result<String> {
        match lex.next_token()? {
            Token::Identifier(s) => Ok(s),
            t => Err(format!("Expected IDENTIFIER, got {:?}", t))?,
        }
    }

    pub fn parse(&mut self, lex: &mut Lexer) -> ParserResult {
        self.parse_file(lex)
    }

    /// Parse a whole file
    fn parse_file(&mut self, lex: &mut Lexer) -> ParserResult {
        loop {
            match lex.next_token()? {
                Token::EOF     => return Ok(()),
                Token::Project => self.parse_project_declaration(lex)?,
                Token::With    => self.parse_with_clause(lex)?,
                t              => Err(format!("Unexpected {:?}", t).to_string())?,
            }
        }
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self, lex: &mut Lexer) -> ParserResult {
        let _path = self.expect_str(lex)?;
        self.expect(lex, Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_declaration(&mut self, lex: &mut Lexer) -> ParserResult {
        let name = self.expect_identifier(lex)?;
        self.parse_project_extension(lex)?;
        self.expect(lex, Token::Is)?;

        self.expect(lex, Token::End)?;
        let endname = self.expect_identifier(lex)?;
        if name != endname {
            return Err(format!("Expected endname {}, got {}", name, endname));
        }
        self.expect(lex, Token::Semicolon)?;
        Ok(())
    }

    fn parse_project_extension(&mut self, lex: &mut Lexer) -> ParserResult {
        self.expect(lex, Token::Extends)?;
        let _extended = self.expect_str(lex)?;
        Ok(())
    }
}
