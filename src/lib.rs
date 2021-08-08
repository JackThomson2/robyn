mod processor;
mod router;
mod server;
mod shared_socket;
mod types;

use server::Server;
use types::Response;

// pyO3 module
use mimalloc::MiMalloc;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use crate::shared_socket::SocketHeld;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[pyfunction]
pub fn start_server() {
    // this is a wrapper function for python
    // to start a server
    Server::new();
}

#[pyfunction]
pub fn prepare_to_run(py: Python) -> PyResult<()> {
    pyo3_asyncio::try_init(py)?;
    pyo3::prepare_freethreaded_python();

    Ok(())
}

#[pymodule]
pub fn robyn(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // the pymodule class to make the rustPyFunctions available
    // in python
    m.add_wrapped(wrap_pyfunction!(start_server))?;
    m.add_wrapped(wrap_pyfunction!(prepare_to_run))?;
    m.add_class::<Server>()?;
    m.add_class::<Response>()?;
    m.add_class::<SocketHeld>()?;
    pyo3_asyncio::try_init(py)?;
    pyo3::prepare_freethreaded_python();
    Ok(())
}
