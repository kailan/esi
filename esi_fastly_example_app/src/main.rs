use fastly::{Error, Request, Response};
use esi_fastly::{process_esi};

pub const BACKEND_NAME: &str = "backend";

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    let mut beresp = req.send(BACKEND_NAME)?;

    process_esi(&mut beresp);

    Ok(beresp)
}
