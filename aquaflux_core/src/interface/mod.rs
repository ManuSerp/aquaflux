pub mod helper;
use crate::interface::helper::extract_expr;
use crate::pipeline;
use pyo3::prelude::*;

/// Python-facing operation types
/// These are separate from internal pipeline types to keep concerns separated

// SINGLE SOURCE OF TRUTH: Define all operations here
// Format: (PyType, PipelineVariant)
macro_rules! define_operations {
    ($($py_type:ty => $variant:ident),* $(,)?) => {
        // Generate the registration list
        pub const ALL_OPERATION_TYPES: &[fn(&Bound<'_, PyModule>) -> PyResult<()>] = &[
            $(|m| { m.add_class::<$py_type>()?; Ok(()) },)*
        ];

        // Generate the extraction function
        pub fn extract_operation(op: &Bound<'_, PyAny>) -> PyResult<pipeline::Op> {
            $(
                if let Ok(extracted) = op.extract::<$py_type>() {
                    return Ok(pipeline::Op::$variant(extracted.into()));
                }
            )*
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Unknown operation type: {}",
                op.get_type().name()?
            )))
        }
    };
}

// here again to we need to use the enum for the op or could we just directly refer to the op itself driectly.
define_operations! {
    PySelectOp => Select,
    PyFillNaOp => FillNa,
    PyCastOp => Cast,
    PyRenameOp => Rename,
    PyDropOp => Drop,
    PyDropNaOp => DropNa,
    PyFilterOp => Filter,
    PyFilterColOp => FilterCol,
    PyGroupByOp => GroupBy,
    PyWithColumns => WithColumns,
}

#[pyclass(name = "SelectOp", from_py_object)]
#[derive(Clone)]
pub struct PySelectOp {
    #[pyo3(get, set)]
    pub columns: Vec<String>,
}

#[pymethods]
impl PySelectOp {
    #[new]
    pub fn new(columns: Vec<String>) -> Self {
        PySelectOp { columns }
    }
}

impl From<PySelectOp> for pipeline::SelectOp {
    fn from(py_op: PySelectOp) -> Self {
        pipeline::SelectOp {
            columns: py_op.columns,
        }
    }
}

#[pyclass(name = "FillNaOp", from_py_object)]
#[derive(Clone)]
pub struct PyFillNaOp {
    #[pyo3(get, set)]
    pub columns: Vec<String>,
    #[pyo3(get, set)]
    pub value: PyScalarValue,
}

#[pymethods]
impl PyFillNaOp {
    #[new]
    pub fn new(columns: Vec<String>, value: PyScalarValueInput) -> Self {
        PyFillNaOp {
            columns,
            value: value.into(),
        }
    }
}

impl From<PyFillNaOp> for pipeline::FillNaOp {
    fn from(py_op: PyFillNaOp) -> Self {
        pipeline::FillNaOp {
            columns: py_op.columns,
            value: py_op.value.into(),
        }
    }
}

#[pyclass(name = "CastOp", from_py_object)]
#[derive(Clone)]
pub struct PyCastOp {
    #[pyo3(get, set)]
    pub columns: Vec<String>,
    #[pyo3(get, set)]
    pub dtype: PyDataType,
}

#[pymethods]
impl PyCastOp {
    #[new]
    pub fn new(columns: Vec<String>, dtype: PyDataTypeInput) -> Self {
        PyCastOp {
            columns,
            dtype: dtype.into(),
        }
    }
}

impl From<PyCastOp> for pipeline::CastOp {
    fn from(py_op: PyCastOp) -> Self {
        pipeline::CastOp {
            columns: py_op.columns,
            dtype: py_op.dtype.into(),
        }
    }
}

#[pyclass(name = "RenameOp", from_py_object)]
#[derive(Clone)]
pub struct PyRenameOp {
    #[pyo3(get, set)]
    pub columns: Vec<String>,
    #[pyo3(get, set)]
    pub new_names: Vec<String>,
}

#[pymethods]
impl PyRenameOp {
    #[new]
    pub fn new(columns: Vec<String>, new_names: Vec<String>) -> Self {
        PyRenameOp { columns, new_names }
    }
}

impl From<PyRenameOp> for pipeline::RenameOp {
    fn from(py_op: PyRenameOp) -> Self {
        pipeline::RenameOp {
            columns: py_op.columns,
            new_names: py_op.new_names,
        }
    }
}

// Enums for Python

#[pyclass(name = "DataType", from_py_object)]
#[derive(Clone)]
pub enum PyDataType {
    Int64,
    Float64,
    String,
    Bool,
}

impl From<PyDataType> for pipeline::DataType {
    fn from(py_dtype: PyDataType) -> Self {
        match py_dtype {
            PyDataType::Int64 => pipeline::DataType::Int64,
            PyDataType::Float64 => pipeline::DataType::Float64,
            PyDataType::String => pipeline::DataType::String,
            PyDataType::Bool => pipeline::DataType::Bool,
        }
    }
}

impl From<PyDataTypeInput> for PyDataType {
    fn from(input: PyDataTypeInput) -> Self {
        match input {
            PyDataTypeInput::Int => PyDataType::Int64,
            PyDataTypeInput::Float => PyDataType::Float64,
            PyDataTypeInput::String => PyDataType::String,
            PyDataTypeInput::Bool => PyDataType::Bool,
            PyDataTypeInput::Wrapped(v) => v,
        }
    }
}

#[derive(Clone)]
pub enum PyDataTypeInput {
    Int,
    Float,
    String,
    Bool,
    Wrapped(PyDataType),
}

impl<'a, 'py> FromPyObject<'a, 'py> for PyDataTypeInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        // Try to extract as wrapped DataType enum first
        if let Ok(wrapped) = ob.extract::<PyDataType>() {
            return Ok(PyDataTypeInput::Wrapped(wrapped));
        }

        // Try to match Python type objects
        let type_name = ob.get_type().name()?;
        let type_name_str = type_name.to_str()?;
        if type_name_str != "type" {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Expected a type object (int, float, str, bool) or DataType enum, got: {}",
                type_name_str
            )));
        }
        if let Ok(py_type) = ob.cast::<pyo3::types::PyType>() {
            let type_name = py_type.name()?;
            match type_name.to_str()? {
                "int" => return Ok(PyDataTypeInput::Int),
                "float" => return Ok(PyDataTypeInput::Float),
                "str" => return Ok(PyDataTypeInput::String),
                "bool" => return Ok(PyDataTypeInput::Bool),
                _ => {}
            }
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected int, float, str, bool type, or DataType enum",
        ))
    }
}

#[pyclass(name = "ScalarValue", from_py_object)]
#[derive(Clone)]
// what really the use of this when we also have the input
pub enum PyScalarValue {
    Int64(i64),
    Float64(f64),
    String(String),
    Bool(bool),
}

impl From<PyScalarValue> for pipeline::ScalarValue {
    fn from(py_value: PyScalarValue) -> Self {
        match py_value {
            PyScalarValue::Int64(v) => pipeline::ScalarValue::Int64(v),
            PyScalarValue::Float64(v) => pipeline::ScalarValue::Float64(v),
            PyScalarValue::String(v) => pipeline::ScalarValue::String(v),
            PyScalarValue::Bool(v) => pipeline::ScalarValue::Bool(v),
        }
    }
}

impl From<PyScalarValueInput> for PyScalarValue {
    fn from(input: PyScalarValueInput) -> Self {
        match input {
            PyScalarValueInput::Int(v) => PyScalarValue::Int64(v),
            PyScalarValueInput::Float(v) => PyScalarValue::Float64(v),
            PyScalarValueInput::String(v) => PyScalarValue::String(v),
            PyScalarValueInput::Bool(v) => PyScalarValue::Bool(v),
            PyScalarValueInput::Wrapped(v) => v,
        }
    }
}

#[derive(Clone)]
pub enum PyScalarValueInput {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Wrapped(PyScalarValue),
}

impl<'a, 'py> FromPyObject<'a, 'py> for PyScalarValueInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        // Try to extract as wrapped type first
        if let Ok(wrapped) = ob.extract::<PyScalarValue>() {
            return Ok(PyScalarValueInput::Wrapped(wrapped));
        }

        // Try primitive types
        if let Ok(s) = ob.extract::<String>() {
            return Ok(PyScalarValueInput::String(s));
        }
        if let Ok(b) = ob.extract::<bool>() {
            return Ok(PyScalarValueInput::Bool(b));
        }
        if let Ok(i) = ob.extract::<i64>() {
            return Ok(PyScalarValueInput::Int(i));
        }
        if let Ok(f) = ob.extract::<f64>() {
            return Ok(PyScalarValueInput::Float(f));
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected int, float, str, bool, or ScalarValue",
        ))
    }
}

#[pyclass(name = "DropOp", from_py_object)]
#[derive(Clone)]
pub struct PyDropOp {
    #[pyo3(get, set)]
    pub columns: Vec<String>,
}

#[pymethods]
impl PyDropOp {
    #[new]
    pub fn new(columns: Vec<String>) -> Self {
        PyDropOp { columns }
    }
}

impl From<PyDropOp> for pipeline::DropOp {
    fn from(py_op: PyDropOp) -> Self {
        pipeline::DropOp {
            columns: py_op.columns,
        }
    }
}

#[pyclass(name = "DropNaOp", from_py_object)]
#[derive(Clone)]
pub struct PyDropNaOp {}

#[pymethods]
impl PyDropNaOp {
    #[new]
    pub fn new() -> Self {
        PyDropNaOp {}
    }
}

impl From<PyDropNaOp> for pipeline::DropNaOp {
    fn from(_: PyDropNaOp) -> Self {
        pipeline::DropNaOp {}
    }
}

#[pyclass(name = "LogicalOp", from_py_object)]
#[derive(Clone)]
pub enum PyLogicalOperator {
    Eq,    // ==
    NotEq, // !=
    Gt,    // >
    Gte,   // >=
    Lt,    // <
    Lte,   // <=
}

impl From<PyLogicalOperator> for pipeline::LogicalOperator {
    fn from(py_lop: PyLogicalOperator) -> Self {
        match py_lop {
            PyLogicalOperator::Eq => pipeline::LogicalOperator::Eq,
            PyLogicalOperator::NotEq => pipeline::LogicalOperator::NotEq,
            PyLogicalOperator::Gt => pipeline::LogicalOperator::Gt,
            PyLogicalOperator::Gte => pipeline::LogicalOperator::Gte,
            PyLogicalOperator::Lt => pipeline::LogicalOperator::Lt,
            PyLogicalOperator::Lte => pipeline::LogicalOperator::Lte,
        }
    }
}

#[pyclass(name = "FilterOp", from_py_object)]
#[derive(Clone)]
pub struct PyFilterOp {
    #[pyo3(get, set)]
    pub column: String,
    #[pyo3(get, set)]
    pub operator: PyLogicalOperator,
    #[pyo3(get, set)]
    pub value: PyScalarValue,
}

#[pymethods]
impl PyFilterOp {
    #[new]
    pub fn new(column: String, operator: PyLogicalOperator, value: PyScalarValueInput) -> Self {
        PyFilterOp {
            column,
            operator,
            value: value.into(),
        }
    }
}

impl From<PyFilterOp> for pipeline::FilterOp {
    fn from(py_op: PyFilterOp) -> Self {
        pipeline::FilterOp {
            column: py_op.column,
            operator: py_op.operator.into(),
            value: py_op.value.into(),
        }
    }
}

#[pyclass(name = "FilterColOp", from_py_object)]
#[derive(Clone)]
pub struct PyFilterColOp {
    pub column: String,
    pub operator: PyLogicalOperator,
    pub other_column: String,
}

#[pymethods]
impl PyFilterColOp {
    #[new]
    pub fn new(column: String, operator: PyLogicalOperator, other_column: String) -> Self {
        PyFilterColOp {
            column,
            operator,
            other_column,
        }
    }
}

impl From<PyFilterColOp> for pipeline::FilterColOp {
    fn from(py_op: PyFilterColOp) -> Self {
        pipeline::FilterColOp {
            column: py_op.column,
            operator: py_op.operator.into(),
            other_column: py_op.other_column,
        }
    }
}

#[pyclass(name = "AggOp", from_py_object)]
#[derive(Clone)]
pub enum PyAggregationFunc {
    Sum,
    Mean,
    Min,
    Max,
    Count,
    Std,
    First,
    Last,
}

impl From<PyAggregationFunc> for pipeline::AggFunction {
    fn from(py_aggop: PyAggregationFunc) -> Self {
        match py_aggop {
            PyAggregationFunc::Sum => pipeline::AggFunction::Sum,
            PyAggregationFunc::Mean => pipeline::AggFunction::Mean,
            PyAggregationFunc::Min => pipeline::AggFunction::Min,
            PyAggregationFunc::Max => pipeline::AggFunction::Max,
            PyAggregationFunc::Count => pipeline::AggFunction::Count,
            PyAggregationFunc::Std => pipeline::AggFunction::Std,
            PyAggregationFunc::First => pipeline::AggFunction::First,
            PyAggregationFunc::Last => pipeline::AggFunction::Last,
        }
    }
}

#[pyclass(name = "GroupByOp", from_py_object)]
#[derive(Clone)]
pub struct PyGroupByOp {
    #[pyo3(get, set)]
    pub group_columns: Vec<String>,
    pub aggregations: Vec<(String, PyAggregationFunc, String)>,
}

#[pymethods]
impl PyGroupByOp {
    #[new]
    pub fn new(
        group_columns: Vec<String>,
        aggregations: Vec<(String, PyAggregationFunc, String)>,
    ) -> Self {
        PyGroupByOp {
            group_columns,
            aggregations,
        }
    }
}

impl From<PyGroupByOp> for pipeline::GroupByOp {
    fn from(py_op: PyGroupByOp) -> Self {
        pipeline::GroupByOp {
            group_columns: py_op.group_columns,
            aggregations: py_op
                .aggregations
                .iter()
                .map(|agg| pipeline::Aggregation {
                    column: agg.0.clone(),
                    function: agg.1.clone().into(),
                    alias: agg.2.clone(),
                })
                .collect(),
        }
    }
}

#[pyclass(name = "Col")]
#[derive(Clone)]
pub struct PyCol {
    pub name: String,
}

#[pymethods]
impl PyCol {
    #[new]
    pub fn new(name: String) -> Self {
        PyCol { name }
    }

    // Col("a") + Col("b") -> Mutation with "a + b"
    // Col("a") + 2 -> Mutation with "a + 2"
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyMut> {
        let other_expr = extract_expr(other)?;
        Ok(PyMut::new(format!("{} + {}", self.name, other_expr), None))
    }

    pub fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyMut> {
        let other_expr = extract_expr(other)?;
        Ok(PyMut::new(format!("{} - {}", self.name, other_expr), None))
    }

    pub fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyMut> {
        let other_expr = extract_expr(other)?;
        Ok(PyMut::new(format!("{} * {}", self.name, other_expr), None))
    }

    pub fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyMut> {
        let other_expr = extract_expr(other)?;
        Ok(PyMut::new(format!("{} / {}", self.name, other_expr), None))
    }
}

// i want to way to do the Mut object: parse string (pseudo compile) or use python operator to straight do it pandas way e,g mut = col1 + col2, mut = col1 * 2
#[pyclass(name = "Mutation", from_py_object)]
#[derive(Clone)]
pub struct PyMut {
    #[pyo3(get, set)]
    pub string_expr: String, //TODO here string is for column, only int or float can be seen as scalar, maybe we need to improve to also handle string as scalar, but for now we will just use string for column name
    #[pyo3(get, set)]
    pub alias: Option<String>,
}
#[pymethods]
impl PyMut {
    #[new]
    pub fn new(string_expr: String, alias: Option<String>) -> Self {
        PyMut { string_expr, alias }
    }
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyMut> {
        let other_expr = extract_expr(other)?;
        Ok(PyMut::new(
            format!("({}) + {}", self.string_expr, other_expr),
            None,
        ))
    }
    // might be a better way to set alias

    pub fn alias(&self, name: String) -> PyMut {
        PyMut::new(self.string_expr.clone(), Some(name))
    }
}

impl From<PyMut> for pipeline::Mutation {
    fn from(py_mut: PyMut) -> Self {
        let mut_expr = crate::interface::helper::string_expr_to_mut_expr(&py_mut.string_expr)
            .expect("Failed to convert string expression to MutExpr");
        pipeline::Mutation {
            expr: mut_expr,
            alias: py_mut.alias,
        }
    }
}

#[pyclass(name = "WithColumns", from_py_object)]
#[derive(Clone)]
pub struct PyWithColumns {
    #[pyo3(get, set)]
    pub mutations: Vec<PyMut>,
}

#[pymethods]
impl PyWithColumns {
    #[new]
    pub fn new(mutations: Vec<PyMut>) -> Self {
        PyWithColumns { mutations }
    }
}

impl From<PyWithColumns> for pipeline::WithColumnsOp {
    fn from(py_op: PyWithColumns) -> Self {
        pipeline::WithColumnsOp {
            mutations: py_op.mutations.into_iter().map(|m| m.into()).collect(),
        }
    }
}
