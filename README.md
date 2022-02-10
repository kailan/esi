# esi

A barebones Rust implementation of Edge Side Includes. Compatible with Fastly Compute@Edge via the [`esi_fastly`](https://docs.rs/esi_fastly) crate.

The goal is to fully implement the [ESI Language Specification 1.0](https://www.w3.org/TR/esi-lang/).

The main branch of this repository was a proof of concept to show that ESI is possible at the edge. This branch aims to be a more robust
implementation, capable of request and response streaming. The API will likely change as this becomes possible.

## Supported Tags

- `<esi:include>` (+ `alt`)
- `<esi:comment>`
- `<esi:remove>`

## Usage

Up-to-date docs coming soon.

## License

The source and documentation for this project are released under the [MIT License](LICENSE).
