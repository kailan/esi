mod parse;
mod config;

use crate::parse::{parse_tags, Tag, Event};
pub use crate::config::Configuration;
use futures::future::{BoxFuture, join};
use futures::{AsyncBufRead, AsyncReadExt, FutureExt};
use http_types::Response;
use quick_xml::Reader;
use std::io::Cursor;
use tokio::sync::mpsc::channel;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("xml parsing error: {0}")]
    XMLError(#[from] quick_xml::Error),
    #[error("tag `{0}` is missing required parameter `{1}`")]
    MissingRequiredParameter(String, String),
    #[error("unexpected `{0}` closing tag")]
    UnexpectedClosingTag(String),
    #[error("duplicate attribute detected: {0}")]
    DuplicateTagAttribute(String),
    #[error("error sending request: {0}")]
    RequestError(String),
    #[error("unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, ExecutionError>;

/// Handles requests to backends as part of the ESI execution process.
/// Implemented by `esi_fastly::FastlyRequestHandler`.
pub trait ExecutionContext<P: PendingRequest> {
    /// Sends a request to the given URL and returns either an error or the response body.
    /// Returns response body.
    fn send_request(&self, url: &str) -> P;
}

pub trait PendingRequest {
    // Block until the request is complete and return the result.
    fn wait(self) -> Result<Response>;
}

pub struct Processor;

impl<'a> Processor {
    pub fn execute_esi<
        E: 'a +  ExecutionContext<P> + Clone + Sync + Send,
        P: PendingRequest + Sync + Send,
        B: 'a + AsyncBufRead + Unpin + Sync + Send,
    >(
        configuration: Configuration,
        client: E,
        mut document: B,
        output: tokio::sync::mpsc::Sender<quick_xml::events::Event<'a>>,
    ) -> BoxFuture<'a, ()> {
        println!("esi execute_esi");

        let (parse_tx, mut parse_rx) = channel(256);
        let namespace = configuration.namespace.clone();

        println!("esi initiated channels");

        let parse_task = async move {
            // TODO: stream
            let mut contents = vec![];
            document.read_to_end(&mut contents).await.unwrap();
            let mut reader = Reader::from_reader(Cursor::new(contents));

            parse_tags(&namespace, &mut reader, parse_tx).await
        };

        println!("esi parse task created");

        let exec_task = async move {
            while let Some(event) = parse_rx.recv().await {
                match event {
                    Event::ESI(Tag::Include { src, alt: _ }) => {
                        let client = client.clone();
                        let configuration = configuration.clone();

                        let pending_request = client.send_request(&src);

                        match pending_request.wait() {
                            Ok(mut resp) => {
                                Processor::execute_esi(
                                    configuration,
                                    client,
                                    resp.take_body().into_reader(),
                                    output.clone(),
                                )
                                .await;
                            }
                            Err(err) => panic!("{:?}", err),
                        }
                    }
                    Event::XML(event) => {
                        println!("sending XML");
                        output.send(event).await.unwrap();
                    }
                }
            }
        };

        let tasks = join(parse_task, exec_task);

        tasks.map(|_| ()).boxed()
    }
}
