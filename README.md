# esi

A barebones Rust implementation of Edge Side Includes. Compatible with Fastly Compute@Edge via the [`esi_fastly`](https://docs.rs/esi_fastly) crate.

## Supported Tags

- `<esi:include>`
- `<esi:comment>`
- `<esi:remove>`

## Usage

### Compute@Edge

#### Cargo.toml

```toml
[dependencies]
esi_fastly = "^0.1"
```

#### src/main.rs

```rust
use fastly::{Error, Request, Response};
use esi_fastly::process_esi;

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Send request to backend.
    let beresp = req.send("backend")?;

    // Process and execute ESI tags within the response body.
    // Make sure you have backends defined for any included hosts.
    // Their names should match the hostname, e.g. "developer.fastly.com"
    let result = process_esi(req, beresp)?;

    // Return the updated response to the client.
    Ok(result)
}
```


### Standalone Rust

#### Cargo.toml

```toml
[dependencies]
esi = "^0.1"
```

#### src/main.rs

```rust
use esi::transform_esi_string;

match transform_esi_string(response_body, &req_handler) {
    Ok(body) => response.set_body(body),
    Err(err) => panic!()
}
```

## License

The source and documentation for this project are released under the [MIT License](LICENSE).
