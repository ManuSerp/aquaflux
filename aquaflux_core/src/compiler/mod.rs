pub mod expr;
use crate::compiler::expr::Expr;
//TODO a true tokenizer would be more complex, but for simplicity, we will just split by whitespace and assume the format is always correct.
//  but in the true parse whitespace would not be necessary to parse the expression, and we would need to handle operator precedence and parentheses correctly.
pub fn parse(expr: &str) -> Result<Expr, String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();

    let left = parse_atom(parts[0])?;
    let op = parts[1];
    let right = parse_atom(parts[2])?;

    Ok(Expr::BinaryOp {
        op: op.to_string(),
        left: Box::new(left),
        right: Box::new(right),
    })
}

//TODO can surely be imrpved as right now any non integer string is treated as a variable, and what about float ?
fn parse_atom(token: &str) -> Result<Expr, String> {
    if let Ok(n) = token.parse::<i64>() {
        Ok(Expr::Literal(n))
    } else if let Ok(n) = token.parse::<f64>() {
        Ok(Expr::FloatLiteral(n))
    } else {
        Ok(Expr::Variable(token.to_string()))
    }
}
