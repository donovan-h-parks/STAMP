//! Vega-Lite's expressions, as far as custom plots use them: in `filter`
//! and `calculate` transforms, for example `datum.count > 10 &&
//! datum.group == 'Gut'` or `log(datum.proportion) / LN10`.
//!
//! The language is a small part of JavaScript: numbers, strings, `true`,
//! `false`, `null`, fields of the row (`datum.name`, `datum['a name']`),
//! arithmetic, comparisons, `!`, `&&`, `||`, `? :`, constants (`PI`, `E`,
//! `LN10`, ...) and common functions (`abs`, `log`, `pow`, `round`,
//! `isValid`, `upper`, ...).

use super::data::Cell;

/// A parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Value(Cell),
    Field(String),
    Unary(char, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Name(String),
    Symbol(&'static str),
}

const SYMBOLS: [&str; 24] = [
    "===", "!==", "==", "!=", "<=", ">=", "&&", "||", "<", ">", "+", "-", "*", "/", "%", "!", "?",
    ":", "(", ")", "[", "]", ".", ",",
];

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || ((chars[i] == 'e' || chars[i] == 'E') && i > start)
                    || ((chars[i] == '-' || chars[i] == '+') && matches!(chars[i - 1], 'e' | 'E')))
            {
                i += 1;
            }
            let number: String = chars[start..i].iter().collect();
            out.push(Token::Number(
                number
                    .parse()
                    .map_err(|_| format!("'{number}' is not a number"))?,
            ));
        } else if c == '\'' || c == '"' {
            let mut text = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err("a string is not closed".to_string()),
                    Some(&q) if q == c => break,
                    Some('\\') => {
                        i += 1;
                        if let Some(&next) = chars.get(i) {
                            text.push(match next {
                                'n' => '\n',
                                't' => '\t',
                                other => other,
                            });
                        }
                    }
                    Some(&other) => text.push(other),
                }
                i += 1;
            }
            i += 1;
            out.push(Token::Text(text));
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else {
            let rest: String = chars[i..chars.len().min(i + 3)].iter().collect();
            let Some(symbol) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) else {
                return Err(format!("unexpected '{c}'"));
            };
            i += symbol.chars().count();
            out.push(Token::Symbol(symbol));
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

/// Binding power of binary operators, lowest first.
fn precedence(op: &str) -> Option<u8> {
    Some(match op {
        "||" => 1,
        "&&" => 2,
        "==" | "!=" | "===" | "!==" => 3,
        "<" | "<=" | ">" | ">=" => 4,
        "+" | "-" => 5,
        "*" | "/" | "%" => 6,
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn expect(&mut self, symbol: &str) -> Result<(), String> {
        match self.next() {
            Some(Token::Symbol(s)) if s == symbol => Ok(()),
            _ => Err(format!("expected '{symbol}'")),
        }
    }

    fn conditional(&mut self) -> Result<Expr, String> {
        let test = self.binary(0)?;
        if self.peek() == Some(&Token::Symbol("?")) {
            self.next();
            let yes = self.conditional()?;
            self.expect(":")?;
            let no = self.conditional()?;
            return Ok(Expr::Conditional(
                Box::new(test),
                Box::new(yes),
                Box::new(no),
            ));
        }
        Ok(test)
    }

    fn binary(&mut self, min: u8) -> Result<Expr, String> {
        let mut left = self.unary()?;
        while let Some(Token::Symbol(op)) = self.peek() {
            let op = *op;
            let Some(power) = precedence(op) else { break };
            // Operators of the same power are taken from the left.
            if power <= min {
                break;
            }
            self.next();
            let right = self.binary(power)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Symbol("!")) => {
                self.next();
                Ok(Expr::Unary('!', Box::new(self.unary()?)))
            }
            Some(Token::Symbol("-")) => {
                self.next();
                Ok(Expr::Unary('-', Box::new(self.unary()?)))
            }
            Some(Token::Symbol("+")) => {
                self.next();
                Ok(Expr::Unary('+', Box::new(self.unary()?)))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Expr::Value(Cell::Num(n))),
            Some(Token::Text(t)) => Ok(Expr::Value(Cell::text(&t))),
            Some(Token::Symbol("(")) => {
                let inner = self.conditional()?;
                self.expect(")")?;
                Ok(inner)
            }
            Some(Token::Name(name)) => self.name(name),
            Some(Token::Symbol(s)) => Err(format!("unexpected '{s}'")),
            None => Err("the expression ends too soon".to_string()),
        }
    }

    fn name(&mut self, name: String) -> Result<Expr, String> {
        if name == "datum" {
            return match self.next() {
                Some(Token::Symbol(".")) => match self.next() {
                    Some(Token::Name(field)) => Ok(Expr::Field(field)),
                    _ => Err("expected a field name after 'datum.'".to_string()),
                },
                Some(Token::Symbol("[")) => {
                    let field = match self.next() {
                        Some(Token::Text(field)) => field,
                        _ => return Err("expected datum['field name']".to_string()),
                    };
                    self.expect("]")?;
                    Ok(Expr::Field(field))
                }
                _ => Err("use datum.field or datum['field name']".to_string()),
            };
        }
        if self.peek() == Some(&Token::Symbol("(")) {
            self.next();
            let mut args = Vec::new();
            if self.peek() != Some(&Token::Symbol(")")) {
                loop {
                    args.push(self.conditional()?);
                    match self.next() {
                        Some(Token::Symbol(",")) => continue,
                        Some(Token::Symbol(")")) => break,
                        _ => return Err(format!("expected ',' or ')' in {name}(...)")),
                    }
                }
            } else {
                self.next();
            }
            if !FUNCTIONS.contains(&name.as_str()) {
                return Err(format!(
                    "unknown function {name}; available: {}",
                    FUNCTIONS.join(", ")
                ));
            }
            return Ok(Expr::Call(name, args));
        }
        let value = match name.as_str() {
            "true" => Cell::Bool(true),
            "false" => Cell::Bool(false),
            "null" | "undefined" => Cell::Null,
            "NaN" => Cell::Num(f64::NAN),
            "Infinity" => Cell::Num(f64::INFINITY),
            "PI" => Cell::Num(std::f64::consts::PI),
            "E" => Cell::Num(std::f64::consts::E),
            "LN10" => Cell::Num(std::f64::consts::LN_10),
            "LN2" => Cell::Num(std::f64::consts::LN_2),
            "LOG10E" => Cell::Num(std::f64::consts::LOG10_E),
            "LOG2E" => Cell::Num(std::f64::consts::LOG2_E),
            "SQRT2" => Cell::Num(std::f64::consts::SQRT_2),
            _ => {
                return Err(format!(
                    "unknown name {name}; fields are written datum.{name}"
                ))
            }
        };
        Ok(Expr::Value(value))
    }
}

const FUNCTIONS: [&str; 22] = [
    "abs", "sqrt", "exp", "log", "log10", "log2", "pow", "min", "max", "round", "floor", "ceil",
    "isValid", "isFinite", "isNaN", "toNumber", "toString", "upper", "lower", "length", "if",
    "indexof",
];

/// Parse an expression.
pub fn parse(text: &str) -> Result<Expr, String> {
    let mut parser = Parser {
        tokens: tokens(text)?,
        at: 0,
    };
    let expr = parser.conditional()?;
    if parser.at < parser.tokens.len() {
        return Err("unexpected text after the expression".to_string());
    }
    Ok(expr)
}

/// Whether a value counts as true, as in JavaScript.
pub fn truthy(cell: &Cell) -> bool {
    match cell {
        Cell::Null => false,
        Cell::Bool(b) => *b,
        Cell::Num(n) => *n != 0.0 && !n.is_nan(),
        Cell::Str(s) => !s.is_empty(),
    }
}

fn equal(a: &Cell, b: &Cell) -> bool {
    match (a, b) {
        (Cell::Null, Cell::Null) => true,
        (Cell::Null, _) | (_, Cell::Null) => false,
        (Cell::Str(x), Cell::Str(y)) => x == y,
        _ => match (a.number(), b.number()) {
            (Some(x), Some(y)) => x == y,
            _ => a.to_text() == b.to_text(),
        },
    }
}

fn compare(a: &Cell, b: &Cell) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Cell::Str(x), Cell::Str(y)) => Some(x.cmp(y)),
        _ => a.number()?.partial_cmp(&b.number()?),
    }
}

/// Evaluate an expression for a row; `field` looks up a field of it.
pub fn eval(expr: &Expr, field: &dyn Fn(&str) -> Cell) -> Cell {
    let num = |e: &Expr| eval(e, field).number().unwrap_or(f64::NAN);
    match expr {
        Expr::Value(v) => v.clone(),
        Expr::Field(name) => field(name),
        Expr::Unary('!', e) => Cell::Bool(!truthy(&eval(e, field))),
        Expr::Unary('-', e) => Cell::Num(-num(e)),
        Expr::Unary(_, e) => Cell::Num(num(e)),
        Expr::Binary(op, a, b) => {
            match *op {
                "&&" => {
                    let left = eval(a, field);
                    return if truthy(&left) { eval(b, field) } else { left };
                }
                "||" => {
                    let left = eval(a, field);
                    return if truthy(&left) { left } else { eval(b, field) };
                }
                _ => {}
            }
            let (x, y) = (eval(a, field), eval(b, field));
            use std::cmp::Ordering::*;
            match *op {
                "==" | "===" => Cell::Bool(equal(&x, &y)),
                "!=" | "!==" => Cell::Bool(!equal(&x, &y)),
                "<" => Cell::Bool(compare(&x, &y) == Some(Less)),
                "<=" => Cell::Bool(matches!(compare(&x, &y), Some(Less | Equal))),
                ">" => Cell::Bool(compare(&x, &y) == Some(Greater)),
                ">=" => Cell::Bool(matches!(compare(&x, &y), Some(Greater | Equal))),
                "+" => match (&x, &y) {
                    (Cell::Str(_), _) | (_, Cell::Str(_)) => {
                        Cell::text(&format!("{}{}", x.to_text(), y.to_text()))
                    }
                    _ => Cell::Num(x.number().unwrap_or(f64::NAN) + y.number().unwrap_or(f64::NAN)),
                },
                _ => {
                    let (p, q) = (
                        x.number().unwrap_or(f64::NAN),
                        y.number().unwrap_or(f64::NAN),
                    );
                    Cell::Num(match *op {
                        "-" => p - q,
                        "*" => p * q,
                        "/" => p / q,
                        _ => p % q,
                    })
                }
            }
        }
        Expr::Conditional(test, yes, no) => {
            if truthy(&eval(test, field)) {
                eval(yes, field)
            } else {
                eval(no, field)
            }
        }
        Expr::Call(name, args) => call(name, args, field),
    }
}

fn call(name: &str, args: &[Expr], field: &dyn Fn(&str) -> Cell) -> Cell {
    let values: Vec<Cell> = args.iter().map(|a| eval(a, field)).collect();
    let n = |i: usize| values.get(i).and_then(Cell::number).unwrap_or(f64::NAN);
    let one = |f: fn(f64) -> f64| Cell::Num(f(n(0)));
    match name {
        "abs" => one(f64::abs),
        "sqrt" => one(f64::sqrt),
        "exp" => one(f64::exp),
        "log" => one(f64::ln),
        "log10" => one(f64::log10),
        "log2" => one(f64::log2),
        "round" => one(f64::round),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "pow" => Cell::Num(n(0).powf(n(1))),
        "min" => Cell::Num((0..values.len()).map(n).fold(f64::INFINITY, f64::min)),
        "max" => Cell::Num((0..values.len()).map(n).fold(f64::NEG_INFINITY, f64::max)),
        "isValid" => Cell::Bool(values.first().is_some_and(Cell::is_valid)),
        "isFinite" => Cell::Bool(n(0).is_finite()),
        "isNaN" => Cell::Bool(n(0).is_nan()),
        "toNumber" => Cell::Num(n(0)),
        "toString" => Cell::text(&values.first().map(Cell::to_text).unwrap_or_default()),
        "upper" => Cell::text(
            &values
                .first()
                .map(Cell::to_text)
                .unwrap_or_default()
                .to_uppercase(),
        ),
        "lower" => Cell::text(
            &values
                .first()
                .map(Cell::to_text)
                .unwrap_or_default()
                .to_lowercase(),
        ),
        "length" => Cell::Num(values.first().map_or(0, |v| v.to_text().chars().count()) as f64),
        "indexof" => {
            let (text, part) = (
                values.first().map(Cell::to_text).unwrap_or_default(),
                values.get(1).map(Cell::to_text).unwrap_or_default(),
            );
            Cell::Num(
                text.find(&part)
                    .map_or(-1.0, |at| text[..at].chars().count() as f64),
            )
        }
        "if" => {
            if values.first().is_some_and(truthy) {
                values.get(1).cloned().unwrap_or(Cell::Null)
            } else {
                values.get(2).cloned().unwrap_or(Cell::Null)
            }
        }
        _ => Cell::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Cell {
        let expr = parse(text).unwrap();
        eval(&expr, &|name| match name {
            "count" => Cell::Num(12.0),
            "group" => Cell::text("Gut"),
            "a name" => Cell::Num(2.0),
            _ => Cell::Null,
        })
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(run("1 + 2 * 3"), Cell::Num(7.0));
        assert_eq!(run("(1 + 2) * 3"), Cell::Num(9.0));
        assert_eq!(run("10 - 4 - 3"), Cell::Num(3.0));
        assert_eq!(run("-datum.count / 4"), Cell::Num(-3.0));
        assert_eq!(run("2e-1 * 10"), Cell::Num(2.0));
        assert_eq!(run("datum['a name'] + 1"), Cell::Num(3.0));
        assert_eq!(run("log10(1000)"), Cell::Num(3.0));
        assert_eq!(run("pow(2, 10)"), Cell::Num(1024.0));
        assert_eq!(run("max(1, 5, 3)"), Cell::Num(5.0));
    }

    #[test]
    fn logic_comparison_and_text() {
        assert_eq!(
            run("datum.count > 10 && datum.group == 'Gut'"),
            Cell::Bool(true)
        );
        assert_eq!(
            run("datum.count > 20 || datum.group === \"Skin\""),
            Cell::Bool(false)
        );
        assert_eq!(run("!(datum.count >= 12)"), Cell::Bool(false));
        assert_eq!(run("datum.count > 5 ? 'many' : 'few'"), Cell::text("many"));
        assert_eq!(run("datum.group + ' ' + datum.count"), Cell::text("Gut 12"));
        assert_eq!(run("isValid(datum.missing)"), Cell::Bool(false));
        assert_eq!(run("upper(datum.group)"), Cell::text("GUT"));
        assert_eq!(run("indexof(datum.group, 'u')"), Cell::Num(1.0));
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(parse("count > 1").unwrap_err().contains("datum.count"));
        assert!(parse("frobnicate(1)")
            .unwrap_err()
            .contains("unknown function"));
        assert!(parse("1 +").is_err());
        assert!(parse("'open").is_err());
        assert!(parse("1 2").is_err());
    }
}
