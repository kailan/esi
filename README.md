# esi

A streaming Edge Side Includes parser and executor designed for Fastly Compute@Edge.

The implementation is currently a subset of the [ESI Language Specification 1.0](https://www.w3.org/TR/esi-lang/).

The main branch of this repository was a proof of concept to show that ESI is possible at the edge. This branch aims to be a more robust
implementation, capable of request and response streaming. The API will likely change as this becomes possible.

## Supported Tags

- `<esi:include>` (+ `alt`, `onerror="continue"`)
- `<esi:comment>`
- `<esi:remove>`

## Usage

Up-to-date docs coming soon.

## License

The source and documentation for this project are released under the [MIT License](LICENSE).
