use fastly::{http::StatusCode, mime, Error, Request, Response};
use esi::Processor;

fn main() {
    if let Err(err) = handle_request(Request::from_client()) {
        println!("returning error response");

        Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            .with_body(err.to_string())
            .send_to_client();
    }
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

    let processor = Processor::new(esi::Configuration::default());

    processor.execute_esi(
        req,
        beresp,
    )?;

    Ok(())
}
