use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse};
use anyhow::Result;
use dashmap::DashMap;
use pyo3::{exceptions::PyValueError, prelude::*};
use pythonize::{depythonize, PythonizeError};
use serde_json::Value;

#[derive(Clone)]
pub enum PyFunction {
    CoRoutine(Py<PyAny>),
    SyncFunction(Py<PyAny>),
}

pub const TEXT: u16 = 1;
pub const STATIC_FILE: u16 = 1;

#[inline]
fn conv_py_to_json_string(v: &Py<PyAny>) -> Result<Value, PythonizeError> {
    let py = unsafe { Python::assume_gil_acquired() };
    depythonize(v.as_ref(py))
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Response {
    pub response_type: u16,
    pub meta: String,
    pub json: Option<Value>,
}

#[pymethods]
impl Response {
    #[inline]
    #[new]
    pub fn new(response_type: u16, meta: String) -> Self {
        Response {
            response_type,
            meta,
            json: None,
        }
    }

    #[inline]
    #[staticmethod]
    pub fn newjson(response_type: u16, _padding: u8, meta: Py<PyAny>) -> PyResult<Self> {
        let data = match conv_py_to_json_string(&meta) {
            Ok(res) => res,
            Err(_e) => return Err(PyValueError::new_err("Cannot parse json")),
        };

        Ok(Response {
            response_type,
            meta: "JSON".to_string(),
            json: Some(data),
        })
    }
}

impl Response {
    #[inline]
    pub fn make_response(&self, req: &HttpRequest) -> Result<HttpResponse> {
        if let Some(json) = &self.json {
            let mut response = HttpResponse::Ok();
            return Ok(response.json(json));
        }

        if self.response_type == STATIC_FILE {
            return Ok(NamedFile::open(&self.meta)?.into_response(req));
        }

        let mut response = HttpResponse::Ok();
        //  apply_headers(&mut response, headers);
        Ok(response.body(&self.meta))
    }
}

pub type Headers = DashMap<String, String>;
