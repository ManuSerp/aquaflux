pub mod dataframe;
use polars::prelude::*;
pub trait Executable {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String>;
}

// I am still unsure about that enum, is a dyn trait better ? (that woulod avoid the need to maintain it)
pub enum Op {
    Select(SelectOp),
    FillNa(FillNaOp),
    Cast(CastOp),
    Rename(RenameOp),
    Drop(DropOp),
    DropNa(DropNaOp),
    Filter(FilterOp),
    FilterCol(FilterColOp),
    GroupBy(GroupByOp),
    WithColumns(WithColumnsOp),
}

impl Executable for Op {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        match self {
            Op::Select(op) => op.execute(df),
            Op::FillNa(op) => op.execute(df),
            Op::Cast(op) => op.execute(df),
            Op::Rename(op) => op.execute(df),
            Op::Drop(op) => op.execute(df),
            Op::DropNa(op) => op.execute(df),
            Op::Filter(op) => op.execute(df),
            Op::FilterCol(op) => op.execute(df),
            Op::GroupBy(op) => op.execute(df),
            Op::WithColumns(op) => op.execute(df),
        }
    }
}
// todo
// waht the point of that, maybe we should remove it and simpl use polars::prelude::DataType directly
#[derive(Clone)]
pub enum DataType {
    Int64,
    Float64,
    String,
    Bool,
}

impl From<DataType> for polars::prelude::DataTypeExpr {
    fn from(dt: DataType) -> Self {
        match dt {
            DataType::Int64 => polars::prelude::DataType::Int64.into(),
            DataType::Float64 => polars::prelude::DataType::Float64.into(),
            DataType::String => polars::prelude::DataType::String.into(),
            DataType::Bool => polars::prelude::DataType::Boolean.into(),
        }
    }
}

#[derive(Clone)]
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
    pub columns: Vec<String>,
    pub dtype: DataType,
}

impl Executable for CastOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let exprs: Vec<Expr> = self
            .columns
            .iter()
            .map(|col_name| col(col_name).cast(self.dtype.clone()))
            .collect();

        df.lazy()
            .with_columns(exprs)
            .collect()
            .map_err(|e| format!("Cast operation failed: {}", e))
    }
}

pub struct RenameOp {
    pub columns: Vec<String>,
    pub new_names: Vec<String>,
}

impl Executable for RenameOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let exprs: Vec<Expr> = self
            .columns
            .iter()
            .zip(self.new_names.iter())
            .map(|(old, new)| col(old).alias(new))
            .collect();
        let temp = df
            .lazy()
            .with_columns(exprs)
            .collect()
            .map_err(|e| format!("Rename operation failed: {}", e))?;
        Ok(temp.drop_many(self.columns.iter()))
    }
}

pub struct DropOp {
    pub columns: Vec<String>,
}

impl Executable for DropOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        Ok(df.drop_many(self.columns.iter()))
    }
}

pub struct DropNaOp {}

impl Executable for DropNaOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        df.drop_nulls(None::<&[&str]>)
            .map_err(|e| format!("Failed to drop nulls: {}", e))
    }
}

pub enum LogicalOperator {
    Eq,    // ==
    NotEq, // !=
    Gt,    // >
    Gte,   // >=
    Lt,    // <
    Lte,   // <=
}

pub struct FilterOp {
    pub column: String,
    pub operator: LogicalOperator,
    pub value: ScalarValue,
}

impl Executable for FilterOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let filter_expr = match self.operator {
            LogicalOperator::Eq => col(&self.column).eq(self.value.scalar_to_expr()),
            LogicalOperator::NotEq => col(&self.column).neq(self.value.scalar_to_expr()),
            LogicalOperator::Gt => col(&self.column).gt(self.value.scalar_to_expr()),
            LogicalOperator::Gte => col(&self.column).gt_eq(self.value.scalar_to_expr()),
            LogicalOperator::Lt => col(&self.column).lt(self.value.scalar_to_expr()),
            LogicalOperator::Lte => col(&self.column).lt_eq(self.value.scalar_to_expr()),
        };

        df.lazy()
            .filter(filter_expr)
            .collect()
            .map_err(|e| format!("Filter operation failed: {}", e))
    }
}

pub struct FilterColOp {
    pub column: String,
    pub operator: LogicalOperator,
    pub other_column: String,
}

impl Executable for FilterColOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let filter_expr = match self.operator {
            LogicalOperator::Eq => col(&self.column).eq(col(&self.other_column)),
            LogicalOperator::NotEq => col(&self.column).neq(col(&self.other_column)),
            LogicalOperator::Gt => col(&self.column).gt(col(&self.other_column)),
            LogicalOperator::Gte => col(&self.column).gt_eq(col(&self.other_column)),
            LogicalOperator::Lt => col(&self.column).lt(col(&self.other_column)),
            LogicalOperator::Lte => col(&self.column).lt_eq(col(&self.other_column)),
        };

        df.lazy()
            .filter(filter_expr)
            .collect()
            .map_err(|e| format!("Filter operation failed: {}", e))
    }
}

pub struct GroupByOp {
    pub group_columns: Vec<String>,
    pub aggregations: Vec<Aggregation>,
}

pub struct Aggregation {
    pub column: String,
    pub function: AggFunction,
    pub alias: String,
}

pub enum AggFunction {
    Sum,
    Mean,
    Min,
    Max,
    Count,
    Std,
    First,
    Last,
}
impl AggFunction {
    pub fn apply(&self, expr: Expr) -> Expr {
        match self {
            Self::Sum => expr.sum(),
            Self::Mean => expr.mean(),
            Self::Min => expr.min(),
            Self::Max => expr.max(),
            Self::Count => expr.count(),
            Self::Std => expr.std(1),
            Self::First => expr.first(),
            Self::Last => expr.last(),
        }
    }
}

impl Executable for GroupByOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let group_exprs: Vec<Expr> = self
            .group_columns
            .iter()
            .map(|col_name| col(col_name))
            .collect();

        let agg_exprs: Vec<Expr> = self
            .aggregations
            .iter()
            .map(|agg| agg.function.apply(col(&agg.column)).alias(&agg.alias))
            .collect();

        df.lazy()
            .group_by(group_exprs)
            .agg(agg_exprs)
            .collect()
            .map_err(|e| format!("Group by operation failed: {}", e))
    }
}

pub enum MutOperator {
    Add,
    Sub,
    Mul,
    Div,
}

impl MutOperator {
    pub fn to_expr(&self, left: Expr, right: Expr) -> Expr {
        match self {
            MutOperator::Add => left + right,
            MutOperator::Sub => left - right,
            MutOperator::Mul => left * right,
            MutOperator::Div => left / right,
        }
    }
}

impl TryFrom<String> for MutOperator {
    type Error = String;

    fn try_from(op: String) -> Result<Self, Self::Error> {
        match op.as_str() {
            "+" => Ok(MutOperator::Add),
            "-" => Ok(MutOperator::Sub),
            "*" => Ok(MutOperator::Mul),
            "/" => Ok(MutOperator::Div),
            _ => Err(format!("Unknown operator: {}", op)),
        }
    }
}

#[derive(Clone)]
pub enum Operand {
    Scalar(ScalarValue),
    Column(String),
}

impl From<Operand> for Expr {
    fn from(op: Operand) -> Self {
        match op {
            Operand::Scalar(scalar) => scalar.scalar_to_expr(),
            Operand::Column(col_name) => col(&col_name),
        }
    }
}

pub struct MutExpr {
    //TODO I dont know if this ever happens in data pipelines but a more correct way would be to recursively have lv and rv as MutExpr (would be the case in compilers)
    pub column: String,
    pub operator: MutOperator,
    pub rv_operand: Operand,
}

pub struct Mutation {
    pub expr: MutExpr,
    pub alias: Option<String>,
}

pub struct WithColumnsOp {
    pub mutations: Vec<Mutation>,
}

impl Executable for WithColumnsOp {
    fn execute(&self, df: dataframe::DataFrame) -> Result<dataframe::DataFrame, String> {
        let mut_exp: Vec<Expr> = self
            .mutations
            .iter()
            .map(|mutation| {
                let right_expr: Expr = mutation.expr.rv_operand.clone().into();

                let base_expr = col(&mutation.expr.column);
                let expr = mutation.expr.operator.to_expr(base_expr, right_expr);

                if let Some(alias) = &mutation.alias {
                    expr.alias(alias)
                } else {
                    expr
                }
            })
            .collect();
        df.lazy()
            .with_columns(mut_exp)
            .collect()
            .map_err(|e| format!("With columns operation failed: {}", e))
    }
}
