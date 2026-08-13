use crate::compiler::expr::Expr;
use crate::interface::{PyCol, PyMut};
use crate::pipeline::{MutExpr, MutOperator, Operand, ScalarValue};
use pyo3::prelude::*;
pub fn extract_expr(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(col) = obj.extract::<PyCol>() {
        return Ok(col.name);
    }
    if let Ok(m) = obj.extract::<PyMut>() {
        return Ok(format!("({})", m.string_expr)); // wrap in parens for precedence
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(i.to_string());
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(f.to_string());
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Expected Col, Mutation, int, or float",
    ))
}

pub fn string_expr_to_mut_expr(string_expr: &str) -> Result<MutExpr, String> {
    // this func take a string expr and return a valid MutExpr
    let expr = crate::compiler::parse(string_expr)?;
    let (op, left, right) = match expr {
        Expr::BinaryOp { op, left, right } => (op, left, right),
        _ => return Err("Expected a binary operation".to_string()),
    };
    let Expr::Variable(column) = *left else {
        return Err("Left side must be a column name".into());
    };

    let mut_op: MutOperator = op.try_into()?;

    let rv_operand: Operand = match *right {
        Expr::Variable(var) => Operand::Column(var),
        Expr::Literal(lit) => Operand::Scalar(ScalarValue::Int64(lit)), //TODO is that correct ? should it not suport float also ? and not sure about the scalar/literal names
        // Also it might also be ok to accept a string on the right side and accept it not as column but a real string literal
        _ => return Err("Right side must be a column name or literal".into()),
    };
    Ok(MutExpr {
        column,
        operator: mut_op,
        rv_operand,
    })
}
