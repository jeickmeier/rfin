//! Parser for Statements DSL formulas.

use crate::dsl::ast::{BinOp, StmtExpr, UnaryOp};
use crate::error::{Error, Result};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{map, opt, recognize},
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

/// Parse a formula string into a `StmtExpr` AST.
///
/// # Example
///
/// ```rust
/// use finstack_statements::dsl::parse_formula;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let ast = parse_formula("revenue - cogs")?;
/// # Ok(())
/// # }
/// ```
pub fn parse_formula(input: &str) -> Result<StmtExpr> {
    match expression(input) {
        Ok(("", expr)) => Ok(expr),
        Ok((remaining, _)) => Err(Error::formula_parse(format!(
            "Unexpected input remaining: '{}'",
            remaining
        ))),
        Err(e) => Err(Error::formula_parse(format!("Parse error: {}", e))),
    }
}

// Expression parser entry point (handles operator precedence)
fn expression(input: &str) -> IResult<&str, StmtExpr> {
    logical_or(input)
}

// Logical OR (lowest precedence)
fn logical_or(input: &str) -> IResult<&str, StmtExpr> {
    let (input, first) = logical_and(input)?;
    let (input, rest) = nom::multi::many0(preceded(
        delimited(multispace0, tag("or"), multispace1),
        logical_and,
    ))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, expr| StmtExpr::bin_op(BinOp::Or, acc, expr)),
    ))
}

// Logical AND
fn logical_and(input: &str) -> IResult<&str, StmtExpr> {
    let (input, first) = comparison(input)?;
    let (input, rest) = nom::multi::many0(preceded(
        delimited(multispace0, tag("and"), multispace1),
        comparison,
    ))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, expr| StmtExpr::bin_op(BinOp::And, acc, expr)),
    ))
}

// Comparison operators
fn comparison(input: &str) -> IResult<&str, StmtExpr> {
    let (input, first) = additive(input)?;

    let (input, opt_op_and_expr) = opt(tuple((
        delimited(
            multispace0,
            alt((
                map(tag("=="), |_| BinOp::Eq),
                map(tag("!="), |_| BinOp::Ne),
                map(tag("<="), |_| BinOp::Le),
                map(tag(">="), |_| BinOp::Ge),
                map(tag("<"), |_| BinOp::Lt),
                map(tag(">"), |_| BinOp::Gt),
            )),
            multispace0,
        ),
        additive,
    )))(input)?;

    match opt_op_and_expr {
        Some((op, second)) => Ok((input, StmtExpr::bin_op(op, first, second))),
        None => Ok((input, first)),
    }
}

// Addition and subtraction
fn additive(input: &str) -> IResult<&str, StmtExpr> {
    let (input, first) = multiplicative(input)?;
    let (input, rest) = nom::multi::many0(tuple((
        delimited(
            multispace0,
            alt((
                map(char('+'), |_| BinOp::Add),
                map(char('-'), |_| BinOp::Sub),
            )),
            multispace0,
        ),
        multiplicative,
    )))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, (op, expr)| StmtExpr::bin_op(op, acc, expr)),
    ))
}

// Multiplication, division, and modulo
fn multiplicative(input: &str) -> IResult<&str, StmtExpr> {
    let (input, first) = unary(input)?;
    let (input, rest) = nom::multi::many0(tuple((
        delimited(
            multispace0,
            alt((
                map(char('*'), |_| BinOp::Mul),
                map(char('/'), |_| BinOp::Div),
                map(char('%'), |_| BinOp::Mod),
            )),
            multispace0,
        ),
        unary,
    )))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, (op, expr)| StmtExpr::bin_op(op, acc, expr)),
    ))
}

// Unary operators
fn unary(input: &str) -> IResult<&str, StmtExpr> {
    alt((
        map(preceded(char('-'), unary), |expr| {
            StmtExpr::unary_op(UnaryOp::Neg, expr)
        }),
        primary,
    ))(input)
}

// Primary expressions (literals, identifiers, function calls, parentheses)
fn primary(input: &str) -> IResult<&str, StmtExpr> {
    delimited(
        multispace0,
        alt((
            if_then_else,
            function_call,
            literal,
            identifier,
            parenthesized,
        )),
        multispace0,
    )(input)
}

// If-then-else expression
fn if_then_else(input: &str) -> IResult<&str, StmtExpr> {
    let (input, _) = tag("if")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, condition) = expression(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(',')(input)?;
    let (input, then_expr) = expression(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(',')(input)?;
    let (input, else_expr) = expression(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((
        input,
        StmtExpr::if_then_else(condition, then_expr, else_expr),
    ))
}

// Function call
fn function_call(input: &str) -> IResult<&str, StmtExpr> {
    let (input, name) = identifier_string(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, args) =
        separated_list0(delimited(multispace0, char(','), multispace0), expression)(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, StmtExpr::call(name, args)))
}

// Literal number
fn literal(input: &str) -> IResult<&str, StmtExpr> {
    map(double, StmtExpr::literal)(input)
}

// Identifier (node reference)
fn identifier(input: &str) -> IResult<&str, StmtExpr> {
    let (input, id_str) = identifier_string(input)?;
    
    // Check if this is a capital structure reference (cs.component.instrument_or_total)
    if id_str.starts_with("cs.") {
        let parts: Vec<&str> = id_str.split('.').collect();
        if parts.len() == 3 {
            // Valid cs reference: cs.component.instrument_or_total
            return Ok((input, StmtExpr::CSRef {
                component: parts[1].to_string(),
                instrument_or_total: parts[2].to_string(),
            }));
        }
    }
    
    Ok((input, StmtExpr::NodeRef(id_str)))
}

// Identifier string (alphanumeric + underscore + dot + hyphen for instrument IDs)
fn identifier_string(input: &str) -> IResult<&str, String> {
    map(
        recognize(pair(
            take_while1(|c: char| c.is_alphabetic() || c == '_'),
            nom::bytes::complete::take_while(|c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '-'),
        )),
        |s: &str| s.to_string(),
    )(input)
}

// Parenthesized expression
fn parenthesized(input: &str) -> IResult<&str, StmtExpr> {
    delimited(char('('), expression, char(')'))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal() {
        let result = parse_formula("42").unwrap();
        assert_eq!(result, StmtExpr::Literal(42.0));

        let result = parse_formula("123.456").unwrap();
        assert_eq!(result, StmtExpr::Literal(123.456));
    }

    #[test]
    fn test_parse_identifier() {
        let result = parse_formula("revenue").unwrap();
        assert_eq!(result, StmtExpr::NodeRef("revenue".into()));
    }

    #[test]
    fn test_parse_addition() {
        let result = parse_formula("1 + 2").unwrap();
        match result {
            StmtExpr::BinOp { op, .. } => assert_eq!(op, BinOp::Add),
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_subtraction() {
        let result = parse_formula("revenue - cogs").unwrap();
        match result {
            StmtExpr::BinOp { op, left, right } => {
                assert_eq!(op, BinOp::Sub);
                assert_eq!(*left, StmtExpr::NodeRef("revenue".into()));
                assert_eq!(*right, StmtExpr::NodeRef("cogs".into()));
            }
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_multiplication() {
        let result = parse_formula("revenue * 0.6").unwrap();
        match result {
            StmtExpr::BinOp { op, .. } => assert_eq!(op, BinOp::Mul),
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_division() {
        let result = parse_formula("gross_profit / revenue").unwrap();
        match result {
            StmtExpr::BinOp { op, .. } => assert_eq!(op, BinOp::Div),
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let result = parse_formula("(1 + 2) * 3").unwrap();
        match result {
            StmtExpr::BinOp {
                op: BinOp::Mul,
                left,
                ..
            } => match *left {
                StmtExpr::BinOp { op: BinOp::Add, .. } => {}
                _ => panic!("Expected Add inside parentheses"),
            },
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse_formula("lag(revenue, 1)").unwrap();
        match result {
            StmtExpr::Call { func, args } => {
                assert_eq!(func, "lag");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Call"),
        }
    }

    #[test]
    fn test_parse_nested_functions() {
        let result = parse_formula("rolling_mean(lag(revenue, 1), 4)").unwrap();
        match result {
            StmtExpr::Call { func, args } => {
                assert_eq!(func, "rolling_mean");
                assert_eq!(args.len(), 2);
                match &args[0] {
                    StmtExpr::Call { func, .. } => assert_eq!(func, "lag"),
                    _ => panic!("Expected nested Call"),
                }
            }
            _ => panic!("Expected Call"),
        }
    }

    #[test]
    fn test_parse_comparison() {
        let result = parse_formula("revenue > 1000000").unwrap();
        match result {
            StmtExpr::BinOp { op, .. } => assert_eq!(op, BinOp::Gt),
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let result = parse_formula("revenue > 1000000 and margin > 0.15").unwrap();
        match result {
            StmtExpr::BinOp { op, .. } => assert_eq!(op, BinOp::And),
            _ => panic!("Expected BinOp"),
        }
    }

    #[test]
    fn test_parse_if_then_else() {
        let result = parse_formula("if(revenue > 1000000, revenue * 0.1, 0)").unwrap();
        match result {
            StmtExpr::IfThenElse { .. } => {}
            _ => panic!("Expected IfThenElse"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let result = parse_formula("(revenue - cogs) / revenue").unwrap();
        match result {
            StmtExpr::BinOp { op: BinOp::Div, .. } => {}
            _ => panic!("Expected division"),
        }
    }

    #[test]
    fn test_parse_negative_number() {
        let result = parse_formula("-5").unwrap();
        match result {
            StmtExpr::UnaryOp {
                op: UnaryOp::Neg, ..
            } => {}
            _ => panic!("Expected unary negation"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // Should parse as 1 + (2 * 3)
        let result = parse_formula("1 + 2 * 3").unwrap();
        match result {
            StmtExpr::BinOp {
                op: BinOp::Add,
                left,
                right,
            } => {
                assert_eq!(*left, StmtExpr::Literal(1.0));
                match *right {
                    StmtExpr::BinOp { op: BinOp::Mul, .. } => {}
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected addition at top level"),
        }
    }

    #[test]
    fn test_parse_error_on_invalid() {
        let result = parse_formula("revenue +");
        assert!(result.is_err());

        let result = parse_formula("revenue @@ cogs");
        assert!(result.is_err());
    }
}
