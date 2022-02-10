use std::str::FromStr;

use esi::{transform_esi_string, Configuration, ExecutionContext, ExecutionError, PendingRequest};
use fastly::{
    http::{header, Url},
    Request, Response,
};

/// A request handler that, given a `fastly::Request`, will route requests to a backend matching
/// the hostname of the request URL.
pub struct FastlyRequestHandler {
    original_req: Request,
}

impl FastlyRequestHandler {
    fn from_request(req: Request) -> FastlyRequestHandler {
        FastlyRequestHandler { original_req: req }
    }
}

struct FastlyPendingRequest(fastly::http::request::PendingRequest);

impl PendingRequest for FastlyPendingRequest {
    fn wait(self: Box<Self>) -> esi::Result<esi::Response> {
        match self.0.wait() {
            Ok(mut resp) => Ok(esi::Response {
                body: resp.take_body_bytes(),
                status_code: resp.get_status().as_u16(),
            }),
            Err(err) => Err(ExecutionError::RequestError(err.to_string())),
        }
    }
}

impl ExecutionContext for FastlyRequestHandler {
    fn send_request(&self, req: esi::Request) -> Box<dyn PendingRequest> {
        println!("Sending request: {:?}", req);

        let mut bereq = self.original_req.clone_without_body().with_url(&req.url);

        // assume that backend name == host
        let parsed_url = Url::from_str(&req.url).unwrap();
        let backend = parsed_url.host_str().unwrap();
        bereq.set_header(header::HOST, backend);

        let pending_request = match bereq.send_async(backend) {
            Ok(resp) => resp,
            Err(_) => panic!("Error sending ESI include request to backend {}", backend),
        };

        let wrapped_request = FastlyPendingRequest(pending_request);

        Box::new(wrapped_request)
    }
}

/// Processes the body of a `fastly::Response` and returns an updated Response after executing
/// all found ESI instructions.
///
/// # Examples
/// ```no_run
/// use fastly::{Error, Request, Response};
/// use esi_fastly::process_esi;
///
/// #[fastly::main]
/// fn main(req: Request) -> Result<Response, Error> {
///     let beresp = req.send("backend")?;
///     process_esi(req, beresp, &esi::Configuration::default());
/// }
/// ```
pub fn process_esi(
    req: Request,
    mut response: Response,
    configuration: &Configuration,
) -> Result<Response, fastly::Error> {
    let req_handler = FastlyRequestHandler::from_request(req);

    match transform_esi_string(response.take_body(), &req_handler, configuration) {
        Ok(body) => response.set_body(body),
        Err(err) => return Err(fastly::Error::from(err)),
    }

    Ok(response)
}
