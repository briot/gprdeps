use crate::base_lexer::{BaseLexer, Context, Lexer};
use crate::errors::Error;
use crate::files::File;
use crate::tokens::TokenKind;
use ustr::Ustr;

pub struct CppLexer<'a> {
    base: BaseLexer<'a>,
}

impl<'a> CppLexer<'a> {
    pub fn new(file: &'a mut File) -> Self {
        Self {
            base: BaseLexer::new(file),
        }
    }

    fn skip_non_tokens(&mut self, current: char) -> char {
        let mut in_comment = false;
        let mut c = current;
        loop {
            match (c, in_comment) {
                ('\n' | ' ' | '\t' | '\r', _) => {}
                ('/', false) => {
                    match self.base.peek_char() {
                        Some('*') => {
                            self.base.scan_char(); // consume '/'
                            self.base.scan_char(); // consume '*'
                            in_comment = true;
                        }
                        Some('/') => self.base.skip_to_eol(),
                        _ => {}
                    }
                }
                ('*', true) => {
                    if let Some('/') = self.base.peek_char() {
                        self.base.scan_char(); //  consume '/'
                        in_comment = false;
                    }
                }
                (_, false) => break,
                _ => {}
            }
            c = self.base.scan_char();
        }
        c
    }

    pub fn skip_to_char(&mut self, marker: char) -> &mut str {
        self.base.skip_to_char(marker)
    }

    fn scan_identifier_or_keyword(&mut self) -> TokenKind {
        match &*self.base.scan_identifier() {
            "loop" => TokenKind::Loop,
            n => TokenKind::Identifier(Ustr::from(n)),
        }
    }

    fn scan_directive(&mut self) -> TokenKind {
        let c = self.base.scan_char(); // consume '#'
        self.skip_non_tokens(c); // There could be spaces

        match &*self.base.scan_identifier() {
            "define" => TokenKind::HashDefine,
            "else" => TokenKind::HashElse,
            "endif" => TokenKind::HashEndif,
            "if" => TokenKind::HashIf,
            "ifdef" => {
                self.base.skip_to_eol();
                TokenKind::HashIfdef
            }
            "ifndef" => TokenKind::HashIfndef,
            "include" => TokenKind::HashInclude,
            "pragma" => TokenKind::Pragma,
            "undef" => TokenKind::HashUndef,
            _ => TokenKind::InvalidChar('#'),
        }
    }
}

impl<'a> Lexer for CppLexer<'a> {
    fn error_with_location(&self, error: Error) -> Error {
        self.base.error_with_location(error)
    }

    fn save_context(&self) -> Context {
        self.base.save_context()
    }

    fn scan_token(&mut self, current: char) -> TokenKind {
        let current = self.skip_non_tokens(current);
        let kind = match current {
            '\x00' => return TokenKind::EndOfFile,
            ')' => TokenKind::CloseParenthesis,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '>' => TokenKind::GreaterThan,
            '<' => TokenKind::LessThan,
            '-' => TokenKind::Minus,
            '(' => TokenKind::OpenParenthesis,
            ';' => TokenKind::Semicolon,
            '#' => return self.scan_directive(),
            '"' => return self.base.scan_quote(),
            _ if self.base.is_wordchar() => {
                return self.scan_identifier_or_keyword();
            }
            c => TokenKind::InvalidChar(c),
        };

        self.base.scan_char();
        kind
    }
}
