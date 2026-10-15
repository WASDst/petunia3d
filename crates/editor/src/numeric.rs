//! Estado puro do controle numérico 3D.
//!
//! O widget Slint apenas apresenta este contrato. Parsing e transições ficam em
//! Rust para que o mesmo comportamento possa ser usado por inspector, painel de
//! operação e testes sem abrir uma janela.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericFieldState {
    value: f32,
    original_value: f32,
    minimum: Option<f32>,
    maximum: Option<f32>,
    step: f32,
    fine_step: f32,
    editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericInputError {
    Empty,
    Invalid,
    NonFinite,
}

impl NumericFieldState {
    pub fn new(value: f32, minimum: Option<f32>, maximum: Option<f32>) -> Self {
        let value = sanitize_value(value);
        Self {
            value,
            original_value: value,
            minimum,
            maximum,
            step: 0.01,
            fine_step: 0.001,
            editing: false,
        }
    }

    pub fn with_steps(mut self, step: f32, fine_step: f32) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        if fine_step.is_finite() && fine_step > 0.0 {
            self.fine_step = fine_step;
        }
        self
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) -> f32 {
        self.value = self.clamp(value);
        self.original_value = self.value;
        self.editing = false;
        self.value
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn begin_edit(&mut self) -> f32 {
        self.original_value = self.value;
        self.editing = true;
        self.value
    }

    pub fn commit_text(&mut self, text: &str) -> Result<f32, NumericInputError> {
        let parsed = parse_numeric_with_base(text, self.value)?;
        self.value = self.clamp(parsed);
        self.original_value = self.value;
        self.editing = false;
        Ok(self.value)
    }

    pub fn cancel(&mut self) -> f32 {
        self.value = self.original_value;
        self.editing = false;
        self.value
    }

    pub fn scrub(&mut self, horizontal_delta: f32, fine: bool) -> f32 {
        if !horizontal_delta.is_finite() {
            return self.value;
        }
        let step = if fine { self.fine_step } else { self.step };
        self.value = self.clamp(self.value + horizontal_delta * step);
        self.value
    }

    fn clamp(&self, value: f32) -> f32 {
        let value = sanitize_value(value);
        let value = self.minimum.map_or(value, |minimum| value.max(minimum));
        self.maximum.map_or(value, |maximum| value.min(maximum))
    }
}

pub fn parse_numeric(text: &str) -> Result<f32, NumericInputError> {
    parse_numeric_with_base(text, 0.0)
}

/// Avalia expressões matemáticas absolutas ou relativas ao valor base (estilo Cinema 4D / CAD).
///
/// Suporta:
/// - Expressões absolutas: `"10 + 5"`, `"180 / 4"`, `"2.5 * 4"`, `"(10 - 2) * 3"`.
/// - Operações relativas: `"+10"`, `"-= 2.5"`, `"*2"`, `"/3"`, `"- 2.5"`.
/// - Variáveis do valor atual: `"x * 2"`, `"v - 1"`, `"# + 5"`.
pub fn parse_numeric_with_base(text: &str, base: f32) -> Result<f32, NumericInputError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(NumericInputError::Empty);
    }

    // Normalização de vírgula decimal para ponto
    let normalized = trimmed.replace(',', ".");
    let norm_str = normalized.trim();

    // Verificação explícita de NaN / Infinity
    let lower = norm_str.to_ascii_lowercase();
    if lower == "nan"
        || lower == "infinity"
        || lower == "+infinity"
        || lower == "-infinity"
        || lower == "inf"
        || lower == "+inf"
        || lower == "-inf"
    {
        return Err(NumericInputError::NonFinite);
    }

    // Prefixo de atribuição composta ou operação relativa direta
    if let Some(rest) = norm_str.strip_prefix("+=") {
        let val = eval_expression(rest, base)?;
        let result = base + val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix("-=") {
        let val = eval_expression(rest, base)?;
        let result = base - val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix("*=") {
        let val = eval_expression(rest, base)?;
        let result = base * val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix("/=") {
        let val = eval_expression(rest, base)?;
        if val.abs() < 1e-12 {
            return Err(NumericInputError::NonFinite);
        }
        let result = base / val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix('*') {
        let val = eval_expression(rest, base)?;
        let result = base * val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix('/') {
        let val = eval_expression(rest, base)?;
        if val.abs() < 1e-12 {
            return Err(NumericInputError::NonFinite);
        }
        let result = base / val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix('+') {
        // Se começa com '+', é adição relativa ao valor base
        let val = eval_expression(rest, base)?;
        let result = base + val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }
    if let Some(rest) = norm_str.strip_prefix("- ") {
        // "- " com espaço explícito é subtração relativa
        let val = eval_expression(rest, base)?;
        let result = base - val;
        return if result.is_finite() {
            Ok(result)
        } else {
            Err(NumericInputError::NonFinite)
        };
    }

    eval_expression(norm_str, base)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f32),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(input: &str, base: f32) -> Result<Vec<Token>, NumericInputError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            'x' | 'X' | 'v' | 'V' | '#' => {
                tokens.push(Token::Number(base));
                i += 1;
            }
            '0'..='9' | '.' => {
                let start = i;
                let mut has_dot = c == '.';
                i += 1;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch.is_ascii_digit() {
                        i += 1;
                    } else if ch == '.' && !has_dot {
                        has_dot = true;
                        i += 1;
                    } else if (ch == 'e' || ch == 'E') && i + 1 < chars.len() {
                        let next = chars[i + 1];
                        if next == '+' || next == '-' || next.is_ascii_digit() {
                            i += 2;
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                i += 1;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                let num_str: String = chars[start..i].iter().collect();
                let val: f32 = num_str.parse().map_err(|_| NumericInputError::Invalid)?;
                if !val.is_finite() {
                    return Err(NumericInputError::NonFinite);
                }
                tokens.push(Token::Number(val));
            }
            _ => return Err(NumericInputError::Invalid),
        }
    }

    if tokens.is_empty() {
        Err(NumericInputError::Empty)
    } else {
        Ok(tokens)
    }
}

struct ExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> ExprParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<&'a Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Result<f32, NumericInputError> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.consume();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.consume();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
            if !left.is_finite() {
                return Err(NumericInputError::NonFinite);
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f32, NumericInputError> {
        let mut left = self.parse_factor()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.consume();
                    let right = self.parse_factor()?;
                    left *= right;
                }
                Token::Slash => {
                    self.consume();
                    let right = self.parse_factor()?;
                    if right.abs() < 1e-12 || !right.is_finite() {
                        return Err(NumericInputError::NonFinite);
                    }
                    left /= right;
                }
                _ => break,
            }
            if !left.is_finite() {
                return Err(NumericInputError::NonFinite);
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<f32, NumericInputError> {
        match self.peek() {
            Some(Token::Plus) => {
                self.consume();
                self.parse_factor()
            }
            Some(Token::Minus) => {
                self.consume();
                let val = self.parse_factor()?;
                Ok(-val)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f32, NumericInputError> {
        match self.consume() {
            Some(Token::Number(val)) => {
                if val.is_finite() {
                    Ok(*val)
                } else {
                    Err(NumericInputError::NonFinite)
                }
            }
            Some(Token::LParen) => {
                let inner = self.parse_expr()?;
                match self.consume() {
                    Some(Token::RParen) => Ok(inner),
                    _ => Err(NumericInputError::Invalid),
                }
            }
            _ => Err(NumericInputError::Invalid),
        }
    }
}

fn eval_expression(expr: &str, base: f32) -> Result<f32, NumericInputError> {
    let tokens = tokenize(expr, base)?;
    let mut parser = ExprParser::new(&tokens);
    let result = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(NumericInputError::Invalid);
    }
    if !result.is_finite() {
        return Err(NumericInputError::NonFinite);
    }
    Ok(result)
}

fn sanitize_value(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_input_commits_and_clamps_to_semantic_range() {
        let mut field = NumericFieldState::new(0.5, Some(0.0), Some(1.0));
        field.begin_edit();
        assert_eq!(field.commit_text(" 1.25 "), Ok(1.0));
        assert!(!field.is_editing());
    }

    #[test]
    fn invalid_input_does_not_corrupt_previous_value() {
        let mut field = NumericFieldState::new(0.5, Some(0.0), Some(1.0));
        field.begin_edit();
        assert_eq!(
            field.commit_text("not-a-number"),
            Err(NumericInputError::Invalid)
        );
        assert_eq!(field.value(), 0.5);
        assert!(field.is_editing());
    }

    #[test]
    fn escape_restores_value_before_edit() {
        let mut field = NumericFieldState::new(2.0, None, None);
        field.begin_edit();
        field.scrub(10.0, false);
        assert_eq!(field.cancel(), 2.0);
        assert!(!field.is_editing());
    }

    #[test]
    fn fine_scrubbing_uses_a_smaller_step() {
        let mut field = NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01);
        assert_eq!(field.scrub(2.0, false), 1.2);
        assert_eq!(field.scrub(2.0, true), 1.22);
    }

    #[test]
    fn setting_a_value_preserves_constraints_and_resets_editing() {
        let mut field = NumericFieldState::new(1.0, Some(0.5), Some(2.0));
        field.begin_edit();

        assert_eq!(field.set_value(4.0), 2.0);
        assert!(!field.is_editing());
        assert_eq!(field.cancel(), 2.0);
    }

    #[test]
    fn parser_rejects_empty_and_non_finite_values() {
        assert_eq!(parse_numeric(""), Err(NumericInputError::Empty));
        assert_eq!(parse_numeric("NaN"), Err(NumericInputError::NonFinite));
        assert_eq!(parse_numeric("Infinity"), Err(NumericInputError::NonFinite));
        assert_eq!(parse_numeric("-0.25"), Ok(-0.25));
    }

    #[test]
    fn arithmetic_expressions_evaluate_correctly() {
        assert_eq!(parse_numeric("10 + 5"), Ok(15.0));
        assert_eq!(parse_numeric("180 / 4"), Ok(45.0));
        assert_eq!(parse_numeric("2.5 * 4"), Ok(10.0));
        assert_eq!(parse_numeric("10 - 2.5 * 2"), Ok(5.0));
        assert_eq!(parse_numeric("(10 - 2.5) * 2"), Ok(15.0));
        assert_eq!(parse_numeric("2,5 + 3,5"), Ok(6.0));
        assert_eq!(parse_numeric("10 / 0"), Err(NumericInputError::NonFinite));
        assert_eq!(parse_numeric("(10 + 5"), Err(NumericInputError::Invalid));
    }

    #[test]
    fn relative_operations_modify_base_value() {
        // Operações diretas relativas
        assert_eq!(parse_numeric_with_base("+10", 5.0), Ok(15.0));
        assert_eq!(parse_numeric_with_base("+ 10", 5.0), Ok(15.0));
        assert_eq!(parse_numeric_with_base("+= 10", 5.0), Ok(15.0));
        assert_eq!(parse_numeric_with_base("-= 2.5", 10.0), Ok(7.5));
        assert_eq!(parse_numeric_with_base("- 2.5", 10.0), Ok(7.5));
        assert_eq!(parse_numeric_with_base("*2", 4.0), Ok(8.0));
        assert_eq!(parse_numeric_with_base("*= 2", 4.0), Ok(8.0));
        assert_eq!(parse_numeric_with_base("/3", 12.0), Ok(4.0));
        assert_eq!(parse_numeric_with_base("/= 3", 12.0), Ok(4.0));

        // Unary minus é absoluto
        assert_eq!(parse_numeric_with_base("-2.5", 10.0), Ok(-2.5));

        // Uso de variável x, v ou #
        assert_eq!(parse_numeric_with_base("x * 2 + 1", 3.0), Ok(7.0));
        assert_eq!(parse_numeric_with_base("v - 2.5", 10.0), Ok(7.5));
        assert_eq!(parse_numeric_with_base("# / 2", 8.0), Ok(4.0));
    }

    #[test]
    fn field_commit_text_supports_expressions_and_relative_mods() {
        let mut field = NumericFieldState::new(10.0, None, None);
        assert_eq!(field.commit_text("+5"), Ok(15.0));
        assert_eq!(field.commit_text("*2"), Ok(30.0));
        assert_eq!(field.commit_text("-= 10"), Ok(20.0));
        assert_eq!(field.commit_text("/4"), Ok(5.0));
        assert_eq!(field.commit_text("x * 2 + 5"), Ok(15.0));
    }
}
