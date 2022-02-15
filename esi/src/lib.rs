use quick_xml::{Reader, Writer};
use std::{
    io::{BufRead, Write},
};
use thiserror::Error;

pub struct Configuration {
    namespace: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            namespace: String::from("esi"),
        }
    }
}

impl Configuration {
    /// Sets an alternative ESI namespace, which is used to identify ESI instructions.
    ///
    /// For example, setting this to `test` would cause the processor to only match tags like `<test:include>`.
    pub fn with_namespace(&mut self, namespace: impl Into<String>) -> &mut Self {
        self.namespace = namespace.into();
        self
    }
}

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
    fn wait(self) -> Result<Box<dyn BufRead>>;
}

/// Representation of an ESI tag from a source response.
#[derive(Debug)]
pub enum Tag {
    Include { src: String, alt: Option<String> },
}

#[derive(Debug)]
pub enum Event<'e> {
    XML(quick_xml::events::Event<'e>),
    ESI(Tag),
}

pub trait OutputSink {
    fn write(&mut self, buf: &[u8]);
}

impl<F: FnMut(&[u8])> OutputSink for F {
    fn write(&mut self, buf: &[u8]) {
        self(buf);
    }
}

impl Write for dyn OutputSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Processor<'a> {
    pub configuration: &'a Configuration,
}

impl<'a> Processor<'a> {
    pub fn execute_esi<E: ExecutionContext<P>, P: PendingRequest>(
        &self,
        client: &E,
        document: Box<dyn BufRead>,
        output: &mut Writer<impl Write>,
    ) {
        let mut reader = Reader::from_reader(document);

        self.parse_tags(&mut reader, |event| {
            match event {
                Event::ESI(Tag::Include { src, alt: _ }) => {
                    let pending_request = client.send_request(&src);

                    // TODO: async + onerror
                    match pending_request.wait() {
                        Ok(resp) => {
                            self.execute_esi(client, resp, output);
                        }
                        Err(err) => panic!("{}", err),
                    }
                }
                Event::XML(event) => {
                    output.write_event(event).unwrap();
                    output.inner().flush().unwrap();
                }
            }
        })
        .unwrap();
    }

    pub fn parse_tags<E: FnMut(Event)>(
        &self,
        reader: &mut Reader<Box<dyn BufRead>>,
        mut events: E,
    ) -> Result<()> {
        let mut remove = false;

        let esi_include = format!("{}:include", self.configuration.namespace).into_bytes();
        let esi_comment = format!("{}:comment", self.configuration.namespace).into_bytes();
        let esi_remove = format!("{}:remove", self.configuration.namespace).into_bytes();

        let mut buffer = Vec::new();
        // Parse tags and build events vec
        loop {
            match reader.read_event(&mut buffer) {
                // Handle <esi:remove> tags
                Ok(quick_xml::events::Event::Start(elem)) if elem.starts_with(&esi_remove) => {
                    remove = true;
                }
                Ok(quick_xml::events::Event::End(elem)) if elem.starts_with(&esi_remove) => {
                    if !remove {
                        return Err(ExecutionError::UnexpectedClosingTag(
                            String::from_utf8(elem.to_vec()).unwrap(),
                        ));
                    }

                    remove = false;
                }
                _ if remove => continue,

                // Handle <esi:include> tags
                Ok(quick_xml::events::Event::Empty(elem))
                    if elem.name().starts_with(&esi_include) =>
                {
                    let mut attributes = elem.attributes().flatten();

                    let src = match attributes.find(|attr| attr.key == b"src") {
                        Some(attr) => String::from_utf8(attr.value.to_vec()).unwrap(),
                        None => {
                            return Err(ExecutionError::MissingRequiredParameter(
                                String::from_utf8(elem.name().to_vec()).unwrap(),
                                "src".to_string(),
                            ));
                        }
                    };

                    let alt = attributes
                        .find(|attr| attr.key == b"alt")
                        .map(|attr| String::from_utf8(attr.value.to_vec()).unwrap());

                    events(Event::ESI(Tag::Include { src, alt }));
                }

                // Ignore <esi:comment> tags
                Ok(quick_xml::events::Event::Empty(elem))
                    if elem.name().starts_with(&esi_comment) =>
                {
                    continue
                }

                Ok(quick_xml::events::Event::Eof) => break,
                Ok(e) => events(Event::XML(e.into_owned())),
                _ => {}
            }
        }

        Ok(())
    }
}
