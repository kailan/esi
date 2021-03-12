use std::str::FromStr;

use esi::{RequestHandler, transform_esi_string};
use fastly::{Request, Response, http::{Url, header}};

struct FastlyRequestHandler {
    original_req: Request
}

impl FastlyRequestHandler {
    fn from_request(req: Request) -> FastlyRequestHandler {
        FastlyRequestHandler {
            original_req: req
        }
    }
}

impl RequestHandler for FastlyRequestHandler {
    fn send_request(&self, url: &str) -> Result<String, esi::Error> {

        let mut bereq = self.original_req.clone_without_body().with_url(url);

        // assume that backend name == host
        let parsed_url = Url::from_str(url).unwrap();
        let backend = parsed_url.host_str().unwrap();
        bereq.set_header(header::HOST, backend);

        let mut beresp = match bereq.send(backend) {
            Ok(resp) => resp,
            Err(_) => panic!("Error sending ESI include request to backend {}", backend)
        };

        Ok(String::from(beresp.take_body_str()))
    }
}

pub fn process_esi(req: Request, mut response: Response) -> Result<Response, fastly::Error> {
    let req_handler = FastlyRequestHandler::from_request(req);

    match transform_esi_string(response.take_body().into_string(), &req_handler) {
        Ok(body) => response.set_body(body),
        Err(err) => return Err(fastly::Error::msg(err.message)),
    }

    Ok(response)
}
