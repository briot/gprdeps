use std::str::Chars;

pub struct Scanner<'a> {
    buffer: &'a str,
    chars: Chars<'a>,
    peeked: Option<char>,
}

#[derive(Debug)]
pub enum Token<'a> {
    Semicolon,
    Identifier(&'a str),
    With,
}

fn is_wordsep(c: Option<char>) -> bool {
    match c {
       None => true,
       Some(' ') => true,
       _         => false,
    }
}

impl<'a> Scanner<'a> {
    pub fn new(buffer: &'a str) -> Self {
        let mut chars = buffer.chars();
        let peeked = chars.next();
        Self {
            buffer,
            chars,
            peeked,
        }
    }

    /// Consumes bytes while a predicate evaluates to true.
    /// Returns the substring read, and the next index after that substring
    fn take_while<F>(data: &str, mut predicate: F) -> Result<(&str, usize)>
        where F: FnMut(char) -> bool
    {
        let mut current_index = 0;
        for c in data.chars() {
            if !predicate(c) {
                break;
            }
            current_index += c.len_utf8();
        }
        if current_index == 0 {
            Err("No Matches".into())
        } else {
            Ok((&data[..current_index], current_index))
        }
    }


    fn next_word(&mut self) -> &'a str {

    }

//    pub fn next_token(&mut self) -> Option<Token> {
//        match self.peeked {
//            None => None,
//            Some('w') => {
//                self.peeked = self.chars.next();
//                if let Some('i') = self.peeker {
//                    self.peeked = self.chars.next();
//                    if let Some('t') = self.peeker {
//                        self.peeked = self.chars.next();
//                        if let Some('h') = self.peeker {
//                            self.peeked = self.chars.next();
//                            if is_wordsep(self.peeked) {
//                                Some(Token::With)
//                            }
//                } else {
//                    None
//                }
//            },
//            Some(';') => Some(Token::Semicolon),
//            _   => None,
//        }
//    }

}
