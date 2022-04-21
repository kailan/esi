use esi_fastly::respond_esi_streaming;
use fastly::{http::StatusCode, mime, Error, Request, Response};
use wasm_rs_async_executor::single_threaded::{spawn, run};

fn main() {
    println!("example app main");
    let _task = spawn(async move {
        if let Err(err) = handle_request(Request::from_client()) {
            println!("returning error response");

            Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                .with_body(err.to_string())
                .send_to_client();
        }
    });
    run(None);
    println!("example app main completed");
}

fn handle_request(req: Request) -> Result<(), Error> {
    println!("example app handle request");

    if req.get_path() != "/" {
        Response::from_status(StatusCode::NOT_FOUND).send_to_client();
        return Ok(());
    }

    // Generate synthetic test response from "index.html" file.
    let beresp = Response::from_body(include_str!("index.html")).with_content_type(mime::TEXT_HTML);

    println!("example app generated beresp");

    respond_esi_streaming(
        req,
        beresp,
        esi::Configuration::default().with_namespace("esi"),
    )
}
