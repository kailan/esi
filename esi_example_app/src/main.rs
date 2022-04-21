use esi::{BackendConfiguration, Processor};
use fastly::{http::StatusCode, mime, Error, Request, Response};

fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Trace)
        .init();

    if let Err(err) = handle_request(Request::from_client()) {
        println!("returning error response");

        Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            .with_body(err.to_string())
            .send_to_client();
    }
}

fn handle_request(req: Request) -> Result<(), Error> {
    if req.get_path() != "/" {
        Response::from_status(StatusCode::NOT_FOUND).send_to_client();
        return Ok(());
    }

    // Generate synthetic test response from "index.html" file.
    let beresp = Response::from_body(include_str!("index.html")).with_content_type(mime::TEXT_HTML);

    let config = esi::Configuration::default()
        .with_backend_override("httpbin.org", "127.0.0.1")
        .with_backend(
            "esi-test.edgecompute.app",
            BackendConfiguration {
                ttl: Some(120),
                ..Default::default()
            },
        );

    let processor = Processor::new(config);

    processor.execute_esi(req, beresp)?;

    Ok(())
}
