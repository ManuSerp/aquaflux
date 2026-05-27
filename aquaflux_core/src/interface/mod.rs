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
