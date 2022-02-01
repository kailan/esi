use fastly::{Error, Request, Response, mime};
use esi_fastly::process_esi_with_namespace;

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Generate synthetic test response from "index.html" file.
    let beresp = Response::from_body(include_str!("index.html")).with_content_type(mime::TEXT_HTML);

    let result = process_esi_with_namespace(req, beresp, String::from("local"))?;

    Ok(result)
}
