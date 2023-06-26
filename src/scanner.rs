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

    pub fn parse(&mut self, lex: &mut Lexer) -> ParserResult {
        self.parse_file(lex)
    }

    /// Parse a whole file
    fn parse_file(&mut self, lex: &mut Lexer) -> ParserResult {
        self.parse_with_clause(lex)
    }

    /// Expect a with_clause
    fn parse_with_clause(&mut self, lex: &mut Lexer) -> ParserResult {
        match lex.next_token()? {
            Token::With => {},
            t => Err(format!("Expected WITH, got {:?}", t).to_string())?,
        };
        match lex.next_token()? {
            Token::String(_) => {},
            t => Err(format!("Expected STRING, got {:?}", t).to_string())?,
        };
        Ok(())
    }
}
