use eyre::Result;

use super::PatuiExpr;

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),
    Number(String),
    LSquareBracket,
    RSquareBracket,
    Dot,
    DotDot,
}

pub(crate) struct Parser<'a> {
    raw: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(raw: &'a str) -> Parser {
        Parser { raw, pos: 0 }
    }

    pub(crate) fn parse(&mut self) -> Result<PatuiExpr> {
        while let Some(token) = self.parse_token() {
            self.handle_token(token);
        }

        todo!();
    }

    fn parse_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let Some(c) = self.peek_char() else {
            return None;
        };

        if c.is_alphabetic() {
            self.parse_ident()
        } else if c.is_digit(10) {
            self.parse_number()
        } else if c == '.' {
            let Some(nc) = self.peek_next_char() else {
                self.pos += 1;
                return Some(Token::Dot);
            };
            if nc != '.' {
                self.pos += 1;
                Some(Token::Dot)
            } else {
                self.pos += 2;
                Some(Token::DotDot)
            }
        } else {
            None
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.raw.chars().nth(self.pos)
    }

    fn peek_next_char(&self) -> Option<char> {
        self.raw.chars().nth(self.pos + 1)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() {
                break;
            }

            self.pos += 1;
        }
    }

    fn parse_number(&mut self) -> Option<Token> {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if start == self.pos && !c.is_digit(10) {
                return None;
            }

            if !c.is_digit(10) && c != '.' {
                break;
            }

            self.pos += 1;
        }

        Some(Token::Number(self.raw[start..self.pos].to_string()))
    }

    fn parse_ident(&mut self) -> Option<Token> {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if start == self.pos && !c.is_alphabetic() && c != '_' {
                return None;
            }

            if !c.is_alphabetic() && !c.is_digit(10) && c != '_' {
                break;
            }

            self.pos += 1;
        }

        Some(Token::Ident(self.raw[start..self.pos].to_string()))
    }

    fn handle_token(&self, token: Token) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn parse_number() {
        let mut parser = Parser::new("123");

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Number("123".to_string()));

        let mut parser = Parser::new("123.45");

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Number("123.45".to_string()));
    }

    #[test]
    fn parse_ident() {
        let mut parser = Parser::new("foo");

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("foo".to_string()));
    }

    #[test]
    fn parse_mixed() {
        let mut parser = Parser::new("bar123.foo");

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("bar123".to_string()));

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Dot);

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("foo".to_string()));
    }

    #[test]
    fn parse_mixed_list() {
        let mut parser = Parser::new("[foo123, bar123.foo, 123]");

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("[".to_string()));

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("foo123".to_string()));

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Ident("bar123".to_string()));

        let ret = parser.parse_token();
        assert_that!(ret).is_some();
        assert_that!(ret.unwrap()).is_equal_to(Token::Number("123".to_string()));
    }
}
