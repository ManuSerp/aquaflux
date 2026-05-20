pub mod dataframe;
use polars::prelude::*;
pub trait Executable {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String>;
}

pub enum Op {
    Select(SelectOp),
    FillNa(FillNaOp),
    Cast(CastOp),
    Rename(RenameOp),
}

impl Executable for Op {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        match self {
            Op::Select(op) => op.execute(df),
            Op::FillNa(op) => op.execute(df),
            Op::Cast(op) => op.execute(df),
            Op::Rename(op) => op.execute(df),
        }
    }
}

pub enum DataType {
    Int64,
    Float64,
    String,
    Bool,
}

pub enum ScalarValue {
    Int64(i64),
    Float64(f64),
    String(String),
    Bool(bool),
}

impl ScalarValue {
    fn scalar_to_expr(&self) -> Expr {
        match self {
            ScalarValue::Int64(v) => lit(*v),
            ScalarValue::Float64(v) => lit(*v),
            ScalarValue::String(v) => lit(v.as_str()),
            ScalarValue::Bool(v) => lit(*v),
        }
    }
}

pub struct SelectOp {
    pub columns: Vec<String>,
}

impl Executable for SelectOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        df.select(&self.columns)
            .map_err(|e| format!("Select operation failed: {}", e))
    }
}

pub struct FillNaOp {
    pub columns: Vec<String>,
    pub value: ScalarValue,
}

impl Executable for FillNaOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let fill_expr = self.value.scalar_to_expr();

        // Create expressions for all columns
        let exprs: Vec<Expr> = self
            .columns
            .iter()
            .map(|col_name| col(col_name).fill_null(fill_expr.clone()))
            .collect();

        // Apply all at once
        df.lazy()
            .with_columns(exprs)
            .collect()
            .map_err(|e| format!("Fill null failed: {}", e))
    }
}

pub struct CastOp {
    pub column: String,
    pub dtype: DataType,
}

impl Executable for CastOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        // dummy implementation, replace with actual logic to cast column types
        Ok(df)
    }
}

pub struct RenameOp {
    pub column: String,
    pub new_name: String,
}

impl Executable for RenameOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        // dummy implementation, replace with actual logic to rename columns
        Ok(df)
    }
}
