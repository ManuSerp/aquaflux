use crate::pipeline;
use pyo3::prelude::*;

/// Python-facing operation types
/// These are separate from internal pipeline types to keep concerns separated

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

// Convert Python type to internal type
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
    pub column: String,
    #[pyo3(get, set)]
    pub value: PyScalarValue,
}

#[pymethods]
impl PyFillNaOp {
    #[new]
    pub fn new(column: String, value: PyScalarValue) -> Self {
        PyFillNaOp { column, value }
    }
}

impl From<PyFillNaOp> for pipeline::FillNaOp {
    fn from(py_op: PyFillNaOp) -> Self {
        pipeline::FillNaOp {
            column: py_op.column,
            value: py_op.value.into(),
        }
    }
}

#[pyclass(name = "CastOp", from_py_object)]
#[derive(Clone)]
pub struct PyCastOp {
    #[pyo3(get, set)]
    pub column: String,
    #[pyo3(get, set)]
    pub dtype: PyDataType,
}

#[pymethods]
impl PyCastOp {
    #[new]
    pub fn new(column: String, dtype: PyDataType) -> Self {
        PyCastOp { column, dtype }
    }
}

impl From<PyCastOp> for pipeline::CastOp {
    fn from(py_op: PyCastOp) -> Self {
        pipeline::CastOp {
            column: py_op.column,
            dtype: py_op.dtype.into(),
        }
    }
}

#[pyclass(name = "RenameOp", from_py_object)]
#[derive(Clone)]
pub struct PyRenameOp {
    #[pyo3(get, set)]
    pub column: String,
    #[pyo3(get, set)]
    pub new_name: String,
}

#[pymethods]
impl PyRenameOp {
    #[new]
    pub fn new(column: String, new_name: String) -> Self {
        PyRenameOp { column, new_name }
    }
}

impl From<PyRenameOp> for pipeline::RenameOp {
    fn from(py_op: PyRenameOp) -> Self {
        pipeline::RenameOp {
            column: py_op.column,
            new_name: py_op.new_name,
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

#[pyclass(name = "ScalarValue", from_py_object)]
#[derive(Clone)]
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
