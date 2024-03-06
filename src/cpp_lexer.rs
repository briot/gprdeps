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
                ('\n' | ' ' | '\t' | '\r' | '\x0c', _) => {}
                ('/', false) => {
                    match self.base.peek_char() {
                        Some('*') => {
                            self.base.scan_char(); // consume '/'
                            self.base.scan_char(); // consume '*'
                            in_comment = true;
                        }
                        Some('/') => {
                            self.base.skip_to_eol();
                        }
                        _ => break,
                    }
                }
                ('*', true) => {
                    if let Some('/') = self.base.peek_char() {
                        self.base.scan_char(); //  consume '/'
                        in_comment = false;
                    }
                }
                ('#', false) => {
                    // Skip all preprocessor directives, except for #include
                    // which we need for dependencies
                    let ctx = self.base.save_context();
                    self.base.scan_char(); //  consume '#'
                    self.base.skip_whitespaces();
                    match &*self.base.scan_identifier() {
                        "include" => {
                            self.base.restore_context(ctx);
                            break;
                        }
                        _ => loop {
                            match self.base.skip_to_eol() {
                                '\\' => {
                                    self.base.scan_char(); // skip newline
                                }
                                '\x00' => return '\x00',
                                _ => break,
                            }
                        },
                    }
                }
                (_, false) => break,
                (_, true) => {}
            }
            c = self.base.scan_char();
        }
        c
    }

    fn scan_identifier_or_keyword(&mut self) -> TokenKind {
        match &*self.base.scan_identifier() {
            "loop" => TokenKind::Loop,
            n => TokenKind::Identifier(Ustr::from(n)),
        }
    }

    fn scan_include(&mut self) -> TokenKind {
        self.base.scan_char(); // consume '#'
        self.base.skip_whitespaces();
        let directive = &*self.base.scan_identifier();
        assert_eq!(directive, "include");
        self.base.skip_whitespaces();
        match self.base.scan_quote() {
            TokenKind::String(n) => TokenKind::HashInclude(n),
            TokenKind::InvalidChar(_) => {
                // sqlite.c has an unusual line
                //    #  include  INC_STRINGIFY(SQLITE_CUSTOM_INCLUDE)
                // Just ignore those for now
                self.base.skip_to_eol();
                TokenKind::HashInclude(Ustr::default())
            }
            _ => panic!("Unexpected path after #include"),
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
            '#' => return self.scan_include(),
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
