use std::io::Read;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io::BufRead, str::FromStr};

use esi::{Configuration, ExecutionContext, ExecutionError, PendingRequest, Processor};
use fastly::{
    http::{header, Url},
    Request, Response,
};
use futures::{AsyncBufRead, AsyncRead};
use futures::future::join;
use quick_xml::Writer;
use tokio::sync::mpsc::channel;
use wasm_rs_async_executor::single_threaded::{block_on, spawn};

/// A request handler that, given a `fastly::Request`, will route requests to a backend matching
/// the hostname of the request URL.
pub struct FastlyRequestHandler {
    original_req: Request,
}

impl Clone for FastlyRequestHandler {
    fn clone(&self) -> Self {
        let mut req = Request::new(self.original_req.get_method(), self.original_req.get_url());
        let mut headers = vec![];
        for header in req.get_header_names() {
            let value = self.original_req.get_header(header).unwrap();
            headers.push((header.to_owned(), value.to_owned()));
        }
        for (k, v) in headers {
            req.set_header(k, v);
        }

        Self { original_req: req }
    }
}

impl FastlyRequestHandler {
    fn from_request(req: Request) -> FastlyRequestHandler {
        FastlyRequestHandler { original_req: req }
    }
}

struct FastlyResponseBody(fastly::http::Body);

impl AsyncRead for FastlyResponseBody {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(self.get_mut().0.read(buf))
    }
}

impl AsyncBufRead for FastlyResponseBody {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        Poll::Ready(self.get_mut().0.fill_buf())
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.get_mut().0.consume(amt)
    }
}

struct FastlyPendingRequest(fastly::http::request::PendingRequest);

impl PendingRequest for FastlyPendingRequest {
    fn wait(self) -> esi::Result<http_types::Response> {
        match self.0.wait() {
            Ok(mut beresp) => {
                let mut resp = http_types::Response::new(beresp.get_status().as_u16());
                resp.set_body(http_types::Body::from_reader(
                    FastlyResponseBody(beresp.take_body()),
                    beresp.get_content_length(),
                ));
                Ok(resp)
            }
            Err(err) => Err(ExecutionError::RequestError(err.to_string())),
        }
    }
}

impl ExecutionContext<FastlyPendingRequest> for FastlyRequestHandler {
    fn send_request(&self, req: &str) -> FastlyPendingRequest {
        println!("Sending request: {:?}", req);

        let mut bereq = self
            .original_req
            .clone_without_body()
            .with_url(req)
            .with_pass(true);

        // assume that backend name == host
        let parsed_url = Url::from_str(req).unwrap();
        let backend = parsed_url.host_str().unwrap();
        bereq.set_header(header::HOST, backend);

        let pending_request = match bereq.send_async(backend) {
            Ok(resp) => resp,
            Err(_) => panic!("Error sending ESI include request to backend {}", backend),
        };

        FastlyPendingRequest(pending_request)
    }
}

pub fn respond_esi_streaming(
    req: Request,
    mut response: Response,
    configuration: Configuration,
) -> Result<(), fastly::Error> {
    println!("esi_fastly respond_esi_streaming");

    let client = FastlyRequestHandler::from_request(req);

    // Take the body from the original ESI document
    let document = response.take_body();

    println!("esi_fastly got body");

    // Send the headers from the original response to the client
    let response = response.stream_to_client();

    let mut writer = Writer::new(response);

    println!("esi_fastly got writer");

    let (tx, mut rx) = channel(256);

    // Transform the body of the original response and stream it to the client
    let exec_task = spawn(Processor::execute_esi(
        configuration,
        client,
        Box::new(FastlyResponseBody(document)),
        tx,
    ));

    let render_task = spawn(async move {
        while let Some(item) = rx.recv().await {
            println!("received XML");
            writer.write_event(item).unwrap();
        }
    });

    println!("esi_fastly execute_esi called");

    let (_, _) = block_on(join(exec_task, render_task));

    println!("esi_fastly block finished");

    Ok(())
}
