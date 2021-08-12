use crate::processor::{apply_headers, handle_request};
use crate::router::{Router, Routing};
use crate::types::Headers;
use std::mem::ManuallyDrop;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::{Arc, Mutex};
use std::thread;
// pyO3 module
use actix_web::*;
use dashmap::DashMap;
use pyo3::types::PyAny;
use pyo3::{prelude::*, GILPool};

// hyper modules
use pyo3_asyncio::run_forever;

static STARTED: AtomicBool = AtomicBool::new(false);

#[pyclass]
pub struct Server {
    router: Arc<Router>,
    headers: Arc<DashMap<String, String>>,
}

#[pymethods]
impl Server {
    #[new]
    pub fn new() -> Self {
        Self {
            router: Arc::new(Router::new()),
            headers: Arc::new(DashMap::new()),
        }
    }

    pub fn start(&mut self, py: Python, port: u16) {
        if STARTED
            .compare_exchange(false, true, SeqCst, Relaxed)
            .is_err()
        {
            println!("Already running...");
            return;
        }

        let router = self.router.clone();
        let headers = self.headers.clone();

        let cores = core_affinity::get_core_ids().unwrap_or_else(Vec::new);
        let cores = Arc::new(Mutex::new(cores));

        thread::spawn(move || {
            //init_current_thread_once();
            actix_web::rt::System::new().block_on(async move {
                let addr = format!("0.0.0.0:{}", port);

                HttpServer::new(move || {
                    if let Some(core) = cores.lock().unwrap().pop() {
                        core_affinity::set_for_current(core);
                    }
                    let _py = ManuallyDrop::new(Python::acquire_gil());

                    App::new()
                        .app_data(web::Data::new(headers.clone()))
                        .app_data(web::Data::new(Routing::new(&router)))
                        .default_service(web::route().to(index))
                })
                .workers(1)
                .bind(addr)
                .unwrap()
                .run()
                .await
                .unwrap();
            });
        });

        run_forever(py).unwrap()
    }

    /// Adds a new header to our concurrent hashmap
    /// this can be called after the server has started.
    pub fn add_header(&self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Removes a new header to our concurrent hashmap
    /// this can be called after the server has started.
    pub fn remove_header(&self, key: &str) {
        self.headers.remove(key);
    }

    /// Add a new route to the routing tables
    /// can be called after the server has been started
    pub fn add_route(&mut self, route_type: &str, route: &str, handler: Py<PyAny>, is_async: bool) {
        println!("Route added for {:4} {} ", route_type, route);
        self.router.add_route(route_type, route, handler, is_async);
    }
}

/// This is our service handler. It receives a Request, routes on it
/// path, and returns a Future of a Response.
#[inline]
async fn index(
    router: web::Data<Routing>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> impl Responder {
    match router.get_route(req.method(), req.uri().path()) {
        Some(handler_function) => {
            match handle_request(handler_function, &mut payload, &req).await {
                Ok(res) => res,
                Err(err) => {
                    println!("Error: {:?}", err);
                    let mut response = HttpResponse::InternalServerError();
                    // apply_headers(&mut response, &headers);
                    response.finish()
                }
            }
        }
        None => {
            let mut response = HttpResponse::NotFound();
            // apply_headers(&mut response, &headers);
            response.finish()
        }
    }
}
