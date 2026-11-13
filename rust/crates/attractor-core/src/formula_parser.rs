//! Recursive-descent parser for the catalog's formula strings.
//!
//! Supported grammar:
//! - numbers (including scientific notation such as `1e-3`) and the variables
//!   `x`, `y`, `z` plus attractor parameter names;
//! - binary operators `+`, `-`, `*`, `/` and right-associative `^`;
//! - unary `+`/`-`;
//! - parentheses;
//! - unary functions listed in [`Expression::SUPPORTED_FUNCTIONS`].
//!
//! This is a port of the Swift `FormulaParser`; evaluation stays in I2.

use crate::formula::{Expression, FormulaError};

/// Maximum nesting depth the recursive-descent parser accepts.
///
/// A real attractor formula nests a handful of levels; the cap only rejects
/// pathological input that would otherwise overflow the call stack (the Swift
/// parser has no cap and aborts on such formulas). It guards parsing and later
/// evaluation, which walks the same tree recursively.
pub const MAX_PARSE_DEPTH: usize = 256;

/// A lexical token produced by the formula tokenizer.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LeftParen,
    RightParen,
}

/// Parse `source` into an expression tree.
pub fn parse(source: &str) -> Result<Expression, FormulaError> {
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return Err(FormulaError::EmptyExpression);
    }
    let mut parser = Parser::new(tokens);
    let expression = parser.parse()?;
    parser.expect_end()?;
    Ok(expression)
}

// MARK: - Tokenizer

fn tokenize(source: &str) -> Result<Vec<Token>, FormulaError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let byte = bytes[i];

        // Whitespace.
        if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
            i += 1;
            continue;
        }

        // Number literal (digits with an optional fractional part and an
        // optional exponent).
        if byte.is_ascii_digit() || byte == b'.' {
            let mut j = i;
            let mut saw_digit = false;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                saw_digit = true;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'.' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    saw_digit = true;
                    j += 1;
                }
            }
            if !saw_digit {
                return Err(FormulaError::UnexpectedToken(friendly(byte)));
            }
            // Exponent: consume `e`/`E` only when at least one digit follows an
            // optional sign, so a bare identifier `e` (a parameter name) still
            // tokenizes on its own.
            if j < bytes.len() && matches!(bytes[j], b'e' | b'E') {
                let mut k = j + 1;
                if k < bytes.len() && matches!(bytes[k], b'+' | b'-') {
                    k += 1;
                }
                let mut saw_exponent_digit = false;
                while k < bytes.len() && bytes[k].is_ascii_digit() {
                    saw_exponent_digit = true;
                    k += 1;
                }
                if saw_exponent_digit {
                    j = k;
                }
            }
            let slice = &source[i..j];
            let value = slice
                .parse::<f64>()
                .map_err(|_| FormulaError::UnexpectedToken(slice.to_owned()))?;
            tokens.push(Token::Number(value));
            i = j;
            continue;
        }

        // Identifier (letters, digits, underscore; may also start with an
        // underscore, as in the Swift tokenizer).
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            tokens.push(Token::Identifier(source[i..j].to_owned()));
            i = j;
            continue;
        }

        // Operators and punctuation.
        tokens.push(match byte {
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'^' => Token::Caret,
            b'(' => Token::LeftParen,
            b')' => Token::RightParen,
            _ => return Err(FormulaError::UnexpectedToken(friendly(byte))),
        });
        i += 1;
    }

    Ok(tokens)
}

/// Render one input byte the way the Swift parser does: as a Latin-1 scalar.
fn friendly(byte: u8) -> String {
    char::from(byte).to_string()
}

// MARK: - Parser

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            depth: 0,
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// Enter one recursion level, refusing input that nests too deeply.
    fn enter_recursion(&mut self) -> Result<(), FormulaError> {
        if self.depth >= MAX_PARSE_DEPTH {
            return Err(FormulaError::TooDeep);
        }
        self.depth += 1;
        Ok(())
    }

    fn leave_recursion(&mut self) {
        self.depth -= 1;
    }

    fn parse(&mut self) -> Result<Expression, FormulaError> {
        self.parse_expression()
    }

    /// Ensure no tokens remain unconsumed.
    fn expect_end(&self) -> Result<(), FormulaError> {
        match self.current() {
            Some(token) => Err(FormulaError::UnexpectedToken(describe(token))),
            None => Ok(()),
        }
    }

    /// `e := t (('+' | '-') t)*`
    fn parse_expression(&mut self) -> Result<Expression, FormulaError> {
        self.enter_recursion()?;
        let result = self.parse_expression_inner();
        self.leave_recursion();
        result
    }

    fn parse_expression_inner(&mut self) -> Result<Expression, FormulaError> {
        let mut lhs = self.parse_term()?;
        while let Some(token) = self.current().cloned() {
            match token {
                Token::Plus => {
                    self.index += 1;
                    lhs = Expression::Add(Box::new(lhs), Box::new(self.parse_term()?));
                }
                Token::Minus => {
                    self.index += 1;
                    lhs = Expression::Subtract(Box::new(lhs), Box::new(self.parse_term()?));
                }
                _ => return Ok(lhs),
            }
        }
        Ok(lhs)
    }

    /// `t := u (('*' | '/') u)*`
    fn parse_term(&mut self) -> Result<Expression, FormulaError> {
        let mut lhs = self.parse_unary()?;
        while let Some(token) = self.current().cloned() {
            match token {
                Token::Star => {
                    self.index += 1;
                    lhs = Expression::Multiply(Box::new(lhs), Box::new(self.parse_unary()?));
                }
                Token::Slash => {
                    self.index += 1;
                    lhs = Expression::Divide(Box::new(lhs), Box::new(self.parse_unary()?));
                }
                _ => return Ok(lhs),
            }
        }
        Ok(lhs)
    }

    /// `u := ('+' | '-') u | p` — unary sign binds looser than `^`.
    fn parse_unary(&mut self) -> Result<Expression, FormulaError> {
        self.enter_recursion()?;
        let result = self.parse_unary_inner();
        self.leave_recursion();
        result
    }

    fn parse_unary_inner(&mut self) -> Result<Expression, FormulaError> {
        match self.current().cloned() {
            None => Err(FormulaError::UnexpectedEndOfInput),
            Some(Token::Plus) => {
                self.index += 1;
                self.parse_unary()
            }
            Some(Token::Minus) => {
                self.index += 1;
                Ok(Expression::Negate(Box::new(self.parse_unary()?)))
            }
            Some(_) => self.parse_power(),
        }
    }

    /// `p := primary ('^' u)?` — right-associative exponentiation.
    fn parse_power(&mut self) -> Result<Expression, FormulaError> {
        let base = self.parse_primary()?;
        if self.current() == Some(&Token::Caret) {
            self.index += 1;
            let exponent = self.parse_unary()?;
            return Ok(Expression::Power(Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expression, FormulaError> {
        let Some(token) = self.current().cloned() else {
            return Err(FormulaError::UnexpectedEndOfInput);
        };
        match token {
            Token::Number(value) => {
                self.index += 1;
                Ok(Expression::Constant(value))
            }
            Token::Identifier(name) => {
                self.index += 1;
                if self.current() == Some(&Token::LeftParen) {
                    self.index += 1;
                    let argument = self.parse_expression()?;
                    if self.current() != Some(&Token::RightParen) {
                        return Err(FormulaError::UnbalancedParenthesis);
                    }
                    self.index += 1;
                    return Ok(Expression::Function(name, Box::new(argument)));
                }
                Ok(Expression::Variable(name))
            }
            Token::LeftParen => {
                self.index += 1;
                let inner = self.parse_expression()?;
                if self.current() != Some(&Token::RightParen) {
                    return Err(FormulaError::UnbalancedParenthesis);
                }
                self.index += 1;
                Ok(inner)
            }
            other => Err(FormulaError::UnexpectedToken(describe(&other))),
        }
    }
}

fn describe(token: &Token) -> String {
    match token {
        Token::Number(value) => value.to_string(),
        Token::Identifier(name) => name.clone(),
        Token::Plus => "+".to_owned(),
        Token::Minus => "-".to_owned(),
        Token::Star => "*".to_owned(),
        Token::Slash => "/".to_owned(),
        Token::Caret => "^".to_owned(),
        Token::LeftParen => "(".to_owned(),
        Token::RightParen => ")".to_owned(),
    }
}
