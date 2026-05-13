pub enum Op {
    Select(SelectOp),
    FillNa(FillNaOp),
    Cast(CastOp),
    Rename(RenameOp),
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

pub struct SelectOp {
    pub columns: Vec<String>,
}

pub struct FillNaOp {
    pub column: String,
    pub value: ScalarValue,
}

pub struct CastOp {
    pub column: String,
    pub dtype: DataType,
}

pub struct RenameOp {
    pub column: String,
    pub new_name: String,
}
