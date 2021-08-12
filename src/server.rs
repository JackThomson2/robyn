use crate::processor::{apply_headers, handle_request};
use crate::router::{Router, Routing};
use crate::shared_socket::SocketHeld;
use crate::types::Headers;
use std::convert::TryInto;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::Arc;
use std::thread;
// pyO3 module
use actix_http::KeepAlive;
use actix_web::*;
use dashmap::DashMap;
use pyo3::prelude::*;
use pyo3::types::PyAny;

// hyper modules
use pyo3_asyncio::{get_event_loop, run_forever, try_init};
use socket2::{Domain, Protocol, Socket, Type};

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

    pub fn start(&mut self, py: Python, socket: &PyCell<SocketHeld>, name: String) -> PyResult<()> {
        if STARTED
            .compare_exchange(false, true, SeqCst, Relaxed)
            .is_err()
        {
            println!("Already running...");
            return Ok(());
        }

        let borrow = socket.try_borrow_mut()?;
        let held_socket: &SocketHeld = &*borrow;

        let raw_socket = held_socket.get_socket();
        println!("Got our socket {:?}", raw_socket);

        let router = self.router.clone();
        let headers = self.headers.clone();

        thread::spawn(move || {
            println!("Thread started...");
            //init_current_thread_once();
            let _res: Result<()> = actix_web::rt::System::new().block_on(async move {
                HttpServer::new(move || {
                    let Gil = Python::acquire_gil();

                    {
                        let py = Gil.python();
                        pyo3_asyncio::try_init(py).unwrap();
                    }

                    App::new()
                        .app_data(web::Data::new(Routing::new(&router.clone())))
                        .app_data(web::Data::new(headers.clone()))
                        .app_data(web::Data::new(Gil))
                        .default_service(web::route().to(index))
                })
                .keep_alive(KeepAlive::Os)
                .workers(1)
                //.max_connection_rate(1)
                .client_timeout(0)
                .listen(raw_socket.try_into()?)?
                .run()
                .await?;

                Ok(())
            });
        });

        Ok(())
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
    pub fn add_route(&self, route_type: &str, route: &str, handler: Py<PyAny>, is_async: bool) {
        println!("Route added for {:4} {} ", route_type, route);
        self.router.add_route(route_type, route, handler, is_async);
    }
}

/// This is our service handler. It receives a Request, routes on it
/// path, and returns a Future of a Response.
#[inline]
async fn index(
    router: web::Data<Routing>,
    headers: web::Data<Arc<Headers>>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> impl Responder {
    match router.get_route(&req.method(), req.uri().path()) {
        Some(handler_function) => {
            match handle_request(handler_function, &headers, &mut payload, &req).await {
                Ok(res) => res,
                Err(err) => {
                    println!("Error: {:?}", err);
                    let mut response = HttpResponse::InternalServerError();
                    apply_headers(&mut response, &headers);
                    response.finish()
                }
            }
        }
        None => {
            let mut response = HttpResponse::NotFound();
            apply_headers(&mut response, &headers);
            response.finish()
        }
    }
}
