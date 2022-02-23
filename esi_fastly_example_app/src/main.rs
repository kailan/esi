use esi_fastly::respond_esi_streaming;
use fastly::{http::StatusCode, mime, Error, Request, Response};

fn main() {
    if let Err(err) = handle_request(Request::from_client()) {
        Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            .with_body(err.to_string())
            .send_to_client();
    }
}

fn handle_request(req: Request) -> Result<(), Error> {
    // Generate synthetic test response from "index.html" file.
    let beresp = Response::from_body(include_str!("index.html")).with_content_type(mime::TEXT_HTML);

    respond_esi_streaming(
        req,
        beresp,
        esi::Configuration::default().with_namespace("esi"),
    )
}
