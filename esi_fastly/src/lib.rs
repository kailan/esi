use esi::transform_esi_string;
use fastly::Response;

pub fn process_esi(mut response: Response) -> Result<Response, fastly::Error> {
    match transform_esi_string(response.take_body().into_string()) {
        Ok(body) => response.set_body(body),
        Err(err) => return Err(fastly::Error::msg(err.message)),
    }

    Ok(response)
}
