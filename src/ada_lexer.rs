use crate::base_lexer::{BaseLexer, Context, Lexer};
use crate::errors::Error;
use crate::files::File;
use crate::tokens::TokenKind;
use ustr::Ustr;

pub struct AdaLexerOptions {
    pub kw_aggregate: bool,
    pub kw_body: bool,
}

pub struct AdaLexer<'a> {
    base: BaseLexer<'a>,
    options: AdaLexerOptions,
}

impl<'a> AdaLexer<'a> {
    pub fn new(file: &'a mut File, options: AdaLexerOptions) -> Self {
        Self {
            base: BaseLexer::new(file),
            options,
        }
    }

    fn skip_non_tokens(&mut self, current: char) -> char {
        let mut c = current;
        loop {
            match c {
                '\n' | ' ' | '\t' | '\r' => {}
                '#' => {
                    // preprocessor
                    self.base.skip_to_eol();
                }
                '-' => {
                    if let Some('-') = self.base.peek_char() {
                        self.base.skip_to_eol();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
            c = self.base.scan_char();
        }
        c
    }

    fn scan_identifier_or_keyword(&mut self) -> TokenKind {
        let n = self.base.scan_identifier();
        n.make_ascii_lowercase();
        match &*n {
            "abstract" => TokenKind::Abstract,
            "aggregate" if self.options.kw_aggregate => TokenKind::Aggregate,
            "body" if self.options.kw_body => TokenKind::Body,
            "case" => TokenKind::Case,
            "end" => TokenKind::End,
            "extends" => TokenKind::Extends,
            "for" => TokenKind::For,
            "function" => TokenKind::Function,
            "generic" => TokenKind::Generic,
            "is" => TokenKind::Is,
            "library" => TokenKind::Library,
            "limited" => TokenKind::Limited,
            "others" => TokenKind::Others,
            "package" => TokenKind::Package,
            "pragma" => TokenKind::Pragma,
            "private" => TokenKind::Private,
            "procedure" => TokenKind::Procedure,
            "project" => TokenKind::Project,
            "renames" => TokenKind::Renames,
            "separate" => TokenKind::Separate,
            "type" => TokenKind::Type,
            "null" => TokenKind::Null,
            "use" => TokenKind::Use,
            "with" => TokenKind::With,
            "when" => TokenKind::When,
            _ => {
                // We can't just do ASCII lower-case, but instead need to do
                // full conversion to lower case here.
                TokenKind::Identifier(Ustr::from(&n.to_lowercase()))
            }
        }
    }
}

impl Lexer for AdaLexer<'_> {
    fn error_with_location(&self, error: Error) -> Error {
        self.base.error_with_location(error)
    }

    fn save_context(&self) -> Context {
        self.base.save_context()
    }

    fn scan_token(&mut self, current: char) -> TokenKind {
        let kind = match self.skip_non_tokens(current) {
            '\x00' => return TokenKind::EndOfFile,
            '&' => TokenKind::Ampersand,
            ')' => TokenKind::CloseParenthesis,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '-' => TokenKind::Minus, // comments handled in skip_non_tokens
            '(' => TokenKind::OpenParenthesis,
            '|' => TokenKind::Pipe,
            ';' => TokenKind::Semicolon,
            '"' => return self.base.scan_quote(),
            '\'' => {
                //  Either a character or a simple tick
                let ctx = self.base.save_context();
                let c = self.base.scan_char();
                if let Some('\'') = self.base.peek_char() {
                    TokenKind::Character(c)
                } else {
                    self.base.restore_context(ctx);
                    TokenKind::Tick
                }
            }
            ':' => {
                if self.base.scan_char() == '=' {
                    self.base.scan_char();
                    return TokenKind::Assign;
                } else {
                    return TokenKind::Colon;
                }
            }
            '=' => {
                if self.base.scan_char() == '>' {
                    self.base.scan_char();
                    return TokenKind::Arrow;
                } else {
                    return TokenKind::Equal;
                }
            }
            _ if self.base.is_wordchar() => {
                return self.scan_identifier_or_keyword();
            }
            c => TokenKind::InvalidChar(c),
        };

        self.base.scan_char();
        kind
    }
}
