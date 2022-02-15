use std::{str::FromStr, io::BufRead};

use esi::{
    Configuration, ExecutionContext, ExecutionError,
    PendingRequest, Processor,
};
use fastly::{
    http::{header, Url},
    Request, Response,
};
use quick_xml::Writer;

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
    fn wait(self) -> esi::Result<Box<dyn BufRead>> {
        match self.0.wait() {
            Ok(mut resp) => Ok(Box::new(resp.take_body())),
            Err(err) => Err(ExecutionError::RequestError(err.to_string())),
        }
    }
}

impl ExecutionContext<FastlyPendingRequest> for FastlyRequestHandler {
    fn send_request(&self, req: &str) -> FastlyPendingRequest {
        println!("Sending request: {:?}", req);

        let mut bereq = self.original_req.clone_without_body().with_url(req);

        // assume that backend name == host
        let parsed_url = Url::from_str(req).unwrap();
        let backend = parsed_url.host_str().unwrap();
        bereq.set_header(header::HOST, backend);

        let pending_request = match bereq.send_async(backend) {
            Ok(resp) => resp,
            Err(_) => panic!("Error sending ESI include request to backend {}", backend),
        };

        FastlyPendingRequest(pending_request)
    }
}

pub fn respond_esi_streaming(
    req: Request,
    mut response: Response,
    configuration: &Configuration,
) -> Result<(), fastly::Error> {
    let client = FastlyRequestHandler::from_request(req);

    let processor = Processor {
        configuration,
    };

    // Take the body from the original ESI document
    let document = response.take_body();

    // Send the headers from the original response to the client
    let response = response.stream_to_client();

    let mut writer = Writer::new(response);

    // Transform the body of the original response and stream it to the client
    processor.execute_esi(&client, Box::new(document), &mut writer);

    Ok(())
}
