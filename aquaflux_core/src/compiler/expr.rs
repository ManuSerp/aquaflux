//TODO polars use some crate with all expression alerady maybe we could reuse it
#[derive(Debug)]
pub enum Expr {
    Variable(String),
    Literal(i64), // can literal could not be allso a stirng and not just i64, float also?
    FloatLiteral(f64),
    BinaryOp {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}
