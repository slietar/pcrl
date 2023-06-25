use pcrl::Value;
use pyo3::prelude::*;


#[pyclass(name = "Error")]
struct PythonError {
    #[pyo3(get)]
    pub message: String,

    #[pyo3(get)]
    pub span: (u32, u32),
}

#[pymethods]
impl PythonError {
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Error({}, range={})",
            repr(py, &self.message.clone().into_py(py))?,
            repr(py, &self.span.clone().into_py(py))?
        ))
    }
}


fn repr(py: Python<'_>, object: &PyObject) -> PyResult<String> {
    Ok(py.import("builtins")?.getattr("repr")?.call1((object,))?.extract::<String>()?)
}


struct ObjectWrapper(pcrl::Object<pcrl::counters::Character>);

impl IntoPy<PyResult<PyObject>> for ObjectWrapper {
    fn into_py(self, py: Python<'_>) -> PyResult<PyObject> {
        match self.0.value {
            Value::Null => Ok(py.None()),
            Value::Bool(value) => Ok(value.into_py(py)),
            Value::String(value) => Ok(value.into_py(py)),
            Value::Float(value) => Ok(value.into_py(py)),
            Value::Integer(value) => Ok(value.into_py(py)),
            Value::List(items) => {
                // let list = pyo3::types::PyList::empty(py);

                // for item in items {
                //     list.append(ObjectWrapper(item).into_py(py)).unwrap();
                // }

                // list.into()

                Ok(items.into_iter().map(|item| ObjectWrapper(item).into_py(py)).collect::<PyResult<Vec<_>>>()?.into_py(py))
            },
            Value::Map(entries) => {
                let dict = pyo3::types::PyDict::new(py);

                for (key, value) in entries {
                    dict.set_item(key.value.into_py(py), ObjectWrapper(value).into_py(py)?)?;
                }

                Ok(dict.into())
            },
        }
    }
}


#[pyfunction]
fn parse(py: Python, text: &str) -> PyResult<PyObject> {
    let result = pcrl::parse::<pcrl::counters::Character>(text);

    let errors = result.errors.into_iter().map(|error| {
        PythonError {
            message: format!("{:?}", error.value),
            span: (error.span.0.counter.position as u32, error.span.1.counter.position as u32),
        }
    }).collect::<Vec<_>>();

    let result_value = match result.object {
        Some(object) => ObjectWrapper(object).into_py(py)?,
        None => py.Ellipsis(),
    };

    Ok((errors, result_value).into_py(py))
}

#[pymodule]
fn pcrllib(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(parse, module)?)?;
    module.add_class::<PythonError>()?;

    Ok(())
}
