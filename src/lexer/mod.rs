pub mod token;

pub use token::{Token, TokenKind, Span, Comment, lookup_keyword, ConnectType};

use crate::utils::error::LexerError;

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    comments: Vec<Comment>,
    last_token_line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            comments: Vec::new(),
            last_token_line: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            // Skip comments (only //)
            if self.peek() == '/' && self.peek_next() == Some('/') {
                self.skip_line_comment();
                continue;
            }

            let token: Token = self.scan_token()?;
            tokens.push(token);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                line: self.line,
                column: self.column,
                start: self.pos,
                end: self.pos,
            },
        });

        Ok(tokens)
    }

    pub fn tokenize_with_comments(&mut self) -> Result<(Vec<Token>, Vec<Comment>), LexerError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            // Collect comments (only //)
            if self.peek() == '/' && self.peek_next() == Some('/') {
                self.collect_comment();
                continue;
            }

            let token: Token = self.scan_token()?;
            self.last_token_line = token.span.line;
            tokens.push(token);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                line: self.line,
                column: self.column,
                start: self.pos,
                end: self.pos,
            },
        });

        let comments = std::mem::take(&mut self.comments);
        Ok((tokens, comments))
    }

    fn collect_comment(&mut self) {
        let comment_line = self.line;
        let comment_col = self.column;
        let is_inline = self.last_token_line == self.line && self.last_token_line > 0;

        self.advance(); // consume first '/'
        self.advance(); // consume second '/'

        // Skip optional leading space after //
        let start = self.pos;
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        let text: String = self.source[start..self.pos].iter().collect();

        self.comments.push(Comment {
            text: text.trim_start().to_string(),
            line: comment_line,
            column: comment_col,
            is_inline,
        });
    }

    fn scan_token(&mut self) -> Result<Token, LexerError> {
        let start = self.pos;
        let start_line = self.line;
        let start_col = self.column;
        let ch = self.advance();

        let kind = match ch {
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            ':' => TokenKind::Colon,
            '#' => TokenKind::Hash,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,

            '-' => {
                if self.peek() == '>' {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }

            '.' => {
                if self.peek() == '.' {
                    self.advance();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }

            '/' => TokenKind::Slash,

            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::EqualEqual
                } else if self.peek() == '>' {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }

            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }

            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }

            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }

            '&' => {
                if self.peek() == '&' {
                    self.advance();
                    TokenKind::And
                } else {
                    return Err(LexerError {
                        message: format!("unexpected character '&', did you mean '&&'?"),
                        span: Span { line: start_line, column: start_col, start, end: self.pos },
                    });
                }
            }

            '|' => {
                if self.peek() == '|' {
                    self.advance();
                    TokenKind::Or
                } else {
                    TokenKind::Pipe
                }
            }

            '"' => return self.read_string(start, start_line, start_col),

            c if c.is_ascii_digit() => return self.read_number(start, start_line, start_col),

            c if c.is_ascii_alphabetic() || c == '_' => {
                return self.read_identifier(start, start_line, start_col);
            }

            other => {
                return Err(LexerError {
                    message: format!("unexpected character '{}'", other),
                    span: Span { line: start_line, column: start_col, start, end: self.pos },
                });
            }
        };

        Ok(Token {
            kind,
            span: Span {
                line: start_line,
                column: start_col,
                start,
                end: self.pos,
            },
        })
    }

    fn read_string(&mut self, start: usize, start_line: usize, start_col: usize) -> Result<Token, LexerError> {
        let mut value = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                return Err(LexerError {
                    message: "unterminated string literal".to_string(),
                    span: Span { line: start_line, column: start_col, start, end: self.pos },
                });
            }
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return Err(LexerError {
                        message: "unterminated escape sequence".to_string(),
                        span: Span { line: start_line, column: start_col, start, end: self.pos },
                    });
                }
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    other => {
                        return Err(LexerError {
                            message: format!("invalid escape sequence '\\{}'", other),
                            span: Span { line: self.line, column: self.column - 2, start: self.pos - 2, end: self.pos },
                        });
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(LexerError {
                message: "unterminated string literal".to_string(),
                span: Span { line: start_line, column: start_col, start, end: self.pos },
            });
        }

        self.advance(); // consume closing "

        Ok(Token {
            kind: TokenKind::StringLiteral(value),
            span: Span { line: start_line, column: start_col, start, end: self.pos },
        })
    }

    fn read_number(&mut self, start: usize, start_line: usize, start_col: usize) -> Result<Token, LexerError> {
        // First char already consumed by scan_token
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        let is_float = !self.is_at_end() && self.peek() == '.'
            && self.peek_next().map_or(false, |c| c.is_ascii_digit());

        if is_float {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let text: String = self.source[start..self.pos].iter().collect();

        let kind = if is_float {
            match text.parse::<f64>() {
                Ok(v) => TokenKind::FloatLiteral(v),
                Err(_) => return Err(LexerError {
                    message: format!("invalid float literal '{}'", text),
                    span: Span { line: start_line, column: start_col, start, end: self.pos },
                }),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => TokenKind::IntLiteral(v),
                Err(_) => return Err(LexerError {
                    message: format!("invalid integer literal '{}'", text),
                    span: Span { line: start_line, column: start_col, start, end: self.pos },
                }),
            }
        };

        Ok(Token {
            kind,
            span: Span { line: start_line, column: start_col, start, end: self.pos },
        })
    }

    fn read_identifier(&mut self, start: usize, start_line: usize, start_col: usize) -> Result<Token, LexerError> {
        // First char already consumed
        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == '_') {
            self.advance();
        }

        let text: String = self.source[start..self.pos].iter().collect();

        let kind = match lookup_keyword(&text) {
            Some(kw) => kw,
            None => TokenKind::Ident(text),
        };

        Ok(Token {
            kind,
            span: Span { line: start_line, column: start_col, start, end: self.pos },
        })
    }

    // --- Helper methods ---

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() { '\0' } else { self.source[self.pos] }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.peek().is_whitespace() {
            self.advance();
        }
    }

    fn skip_line_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(input);
        lexer.tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_let_statement() {
        let kinds = tokenize("let x: int = 5;");
        assert_eq!(kinds, vec![
            TokenKind::Let,
            TokenKind::Ident("x".to_string()),
            TokenKind::Colon,
            TokenKind::IntType,
            TokenKind::Equal,
            TokenKind::IntLiteral(5),
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_function_declaration() {
        let kinds = tokenize("fn add(a: int, b: int) -> int { }");
        assert_eq!(kinds, vec![
            TokenKind::Fn,
            TokenKind::Ident("add".to_string()),
            TokenKind::LeftParen,
            TokenKind::Ident("a".to_string()),
            TokenKind::Colon,
            TokenKind::IntType,
            TokenKind::Comma,
            TokenKind::Ident("b".to_string()),
            TokenKind::Colon,
            TokenKind::IntType,
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::IntType,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_operators() {
        let kinds = tokenize("== != <= >= && || + - * / %");
        assert_eq!(kinds, vec![
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_string_with_escapes() {
        let kinds = tokenize(r#""hello\nworld""#);
        assert_eq!(kinds, vec![
            TokenKind::StringLiteral("hello\nworld".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_float_literal() {
        let kinds = tokenize("3.14");
        assert_eq!(kinds, vec![
            TokenKind::FloatLiteral(3.14),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_for_range() {
        let kinds = tokenize("for i in 0..10 { }");
        assert_eq!(kinds, vec![
            TokenKind::For,
            TokenKind::Ident("i".to_string()),
            TokenKind::In,
            TokenKind::IntLiteral(0),
            TokenKind::DotDot,
            TokenKind::IntLiteral(10),
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_comments_ignored() {
        let kinds = tokenize("let x: int = 5; // this is a comment");
        assert_eq!(kinds, vec![
            TokenKind::Let,
            TokenKind::Ident("x".to_string()),
            TokenKind::Colon,
            TokenKind::IntType,
            TokenKind::Equal,
            TokenKind::IntLiteral(5),
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_booleans_and_keywords() {
        let kinds = tokenize("true false let mut struct");
        assert_eq!(kinds, vec![
            TokenKind::BoolLiteral(true),
            TokenKind::BoolLiteral(false),
            TokenKind::Let,
            TokenKind::Mut,
            TokenKind::Struct,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new("\"hello");
        assert!(lexer.tokenize().is_err());
    }

    #[test]
    fn test_unexpected_character() {
        let mut lexer = Lexer::new("$");
        assert!(lexer.tokenize().is_err());
    }
}
