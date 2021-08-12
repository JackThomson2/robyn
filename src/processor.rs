use actix_web::{http::Method, web, HttpRequest, HttpResponse, HttpResponseBuilder};
use anyhow::{bail, Result};
use std::sync::Arc;
// pyO3 module
use crate::types::{Headers, PyFunction, Response};
use futures_util::stream::StreamExt;
use pyo3::{prelude::*, GILPool};

/// @TODO make configurable
const MAX_SIZE: usize = 10_000;

#[inline]
pub fn apply_headers(response: &mut HttpResponseBuilder, headers: &Arc<Headers>) {
    for a in headers.iter() {
        response.insert_header((a.key().clone(), a.value().clone()));
    }
}

/// This functions handles the incoming request matches it to the function and serves the response
///
/// # Arguments
///
/// * `function` - a PyFunction matched from the router
///
/// # Errors
///
/// When the route is not found. It should check if the 404 route exist and then serve it back
/// There can also be PyError due to any mis processing of the files
///
#[inline(always)]
pub async fn handle_request(
    function: &PyFunction,
    payload: &mut web::Payload,
    req: &HttpRequest,
) -> Result<HttpResponse> {
    let mut data: Option<Vec<u8>> = None;

    if req.method() == Method::POST {
        let mut body = web::BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let chunk = chunk?;
            // limit max size of in-memory payload
            if (body.len() + chunk.len()) > MAX_SIZE {
                bail!("Overflow");
            }
            body.extend_from_slice(&chunk);
        }

        data = Some(body.to_vec())
    }

    match function {
        PyFunction::CoRoutine(handler) => {
            let py = unsafe { Python::assume_gil_acquired() };
            let output = {
                let handler = handler.as_ref(py);

                let coro: PyResult<&PyAny> = match data {
                    Some(res) => {
                        let data = res.into_py(py);
                        handler.call1((&data,))
                    }
                    None => handler.call0(),
                };
                pyo3_asyncio::into_future(coro?)
            }?;

            let output = output.await?;
            let reffer: Response = output.extract(py)?;
            reffer.make_response(req)
        }
        PyFunction::SyncFunction(handler) => {
            let py = unsafe { Python::assume_gil_acquired() };

            let res: Py<PyAny> = match data {
                Some(res) => {
                    let data = res.into_py(py);
                    handler.call1(py, (&data,))
                }
                None => handler.call0(py),
            }?;

            let response: Response = res.extract(py)?;
            response.make_response(req)
        }
    }
}
