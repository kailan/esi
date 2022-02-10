use esi_fastly::process_esi;
use fastly::{mime, Error, Request, Response};

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Generate synthetic test response from "index.html" file.
    let beresp = Response::from_body(include_str!("index.html")).with_content_type(mime::TEXT_HTML);

    let result = process_esi(
        req,
        beresp,
        esi::Configuration::default().with_namespace("local"),
    )?;

    Ok(result)
}
