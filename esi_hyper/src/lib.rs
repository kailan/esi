use futures::executor::block_on;
use hyper::client::ResponseFuture;
use hyper::{Body, Error, Request, Response};
use std::io::{BufRead, BufReader};

use esi::{Configuration, ExecutionContext, ExecutionError, PendingRequest, Processor};

/// A request handler that, given a `hyper::Request`, will create backend requests
/// using the original request headers.
pub struct HyperRequestHandler {
    original_req: Request<Body>,
}

impl HyperRequestHandler {
    fn from_request(req: Request<Body>) -> HyperRequestHandler {
        HyperRequestHandler { original_req: req }
    }
}

struct HyperPendingRequest(ResponseFuture);

impl PendingRequest for HyperPendingRequest {
    fn wait(self) -> esi::Result<http_types::response::Response> {
        todo!()
    }
}

impl ExecutionContext<HyperPendingRequest> for HyperRequestHandler {
    fn send_request(&self, _req: &str) -> HyperPendingRequest {
        todo!()
    }
}

// pub fn respond_esi_streaming(
//     req: Request<Body>,
//     mut response: Response<Body>,
//     configuration: &Configuration,
// ) -> Result<(), Error> {
//     let client = HyperRequestHandler::from_request(req);
//
//     let processor = Processor { configuration };
//
//     // Take the body from the original ESI document
//     let document = response.into_body();
//
//     // Send the headers from the original response to the client
//     let response = response.
//
//     let mut writer = Writer::new(response);
//
//     // Transform the body of the original response and stream it to the client
//     processor.execute_esi(&client, Box::new(document), &mut writer);
//
//     Ok(())
// }
