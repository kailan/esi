use quick_xml::{
    events::{BytesText, Event},
    Reader, Writer,
};
use std::{collections::HashMap, io::BufRead};
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

/// A request initiated by the ESI executor.
#[derive(Debug)]
pub struct Request {
    pub url: String,
}

impl Request {
    fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }
}

/// A response from the local `ExecutionContext` implementation.
/// Usually the result of a `Request`.
#[derive(Debug)]
pub struct Response {
    pub body: Vec<u8>,
    pub status_code: u16,
}

/// Handles requests to backends as part of the ESI execution process.
/// Implemented by `esi_fastly::FastlyRequestHandler`.
pub trait ExecutionContext {
    /// Sends a request to the given URL and returns either an error or the response body.
    /// Returns response body.
    fn send_request(&self, req: Request) -> Box<dyn PendingRequest>;
}

pub trait PendingRequest {
    // Block until the request is complete and return the result.
    fn wait(self: Box<Self>) -> Result<Response>;
}

/// Representation of an ESI tag from a source response.
#[derive(Debug)]
pub enum Tag {
    Include { src: String, alt: Option<String> },
}

pub struct TagEntry<'a> {
    event: Option<Event<'a>>,
    esi_tag: Option<Tag>,
}

fn parse_tag_entries<'a>(
    body: impl BufRead,
    configuration: &Configuration,
) -> Result<Vec<TagEntry<'a>>> {
    let mut reader = Reader::from_reader(body);
    let mut buf = Vec::new();

    let mut events: Vec<TagEntry> = Vec::new();
    let mut remove = false;

    let esi_include = format!("{}:include", configuration.namespace).into_bytes();
    let esi_comment = format!("{}:comment", configuration.namespace).into_bytes();
    let esi_remove = format!("{}:remove", configuration.namespace).into_bytes();

    // Parse tags and build events vec
    loop {
        buf.clear();
        match reader.read_event(&mut buf) {
            // Handle <esi:remove> tags
            Ok(Event::Start(elem)) if elem.starts_with(&esi_remove) => {
                remove = true;
            }
            Ok(Event::End(elem)) if elem.starts_with(&esi_remove) => {
                if !remove {
                    return Err(ExecutionError::UnexpectedClosingTag(
                        String::from_utf8(elem.to_vec()).unwrap(),
                    ));
                }

                remove = false;
            }
            _ if remove => continue,

            // Handle <esi:include> tags
            Ok(Event::Empty(elem)) if elem.name().starts_with(&esi_include) => {
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

                events.push(TagEntry {
                    event: None,
                    esi_tag: Some(Tag::Include { src, alt }),
                });
            },

            // Ignore <esi:comment> tags
            Ok(Event::Empty(elem)) if elem.name().starts_with(&esi_comment) => continue,

            Ok(Event::Eof) => break,
            Ok(e) => events.push(TagEntry {
                event: Some(e.into_owned()),
                esi_tag: None,
            }),
            _ => {}
        }
    }

    Ok(events)
}

// Executes all entries with an ESI tag, and returns a map of those entries with the entry's index as key and content as value.
fn execute_tag_entries(
    entries: &[TagEntry],
    client: &impl ExecutionContext,
    _configuration: &Configuration,
) -> Result<HashMap<usize, Vec<u8>>> {
    let mut map = HashMap::new();

    for (index, entry) in entries.iter().enumerate() {
        match &entry.esi_tag {
            Some(Tag::Include { src, alt }) => {
                let pending_request = client.send_request(Request::from_url(src));

                // TODO: async + onerror
                match pending_request.wait() {
                    Ok(resp) => {
                        map.insert(index, resp.body);
                    }
                    Err(err) => match alt {
                        Some(alt) => {
                            let pending_request = client.send_request(Request::from_url(alt));

                            match pending_request.wait() {
                                Ok(resp) => {
                                    map.insert(index, resp.body);
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        None => {
                            return Err(err);
                        }
                    },
                }
            },
            None => {}
        }
    }

    Ok(map)
}

/// Processes a given ESI response body and returns the transformed body after all ESI instructions
/// have been executed.
pub fn transform_esi_string(
    body: impl BufRead,
    client: &impl ExecutionContext,
    configuration: &Configuration,
) -> Result<Vec<u8>> {
    // Parse tags
    let events = parse_tag_entries(body, configuration)?;

    // Execute tags
    let results = execute_tag_entries(&events, client, configuration)?;

    // Build output XML
    let mut writer = Writer::new(Vec::new());

    for (index, entry) in events.iter().enumerate() {
        match &entry.esi_tag {
            Some(_tag) => {
                if let Some(content) = results.get(&index) {
                    writer
                        .write_event(Event::Text(BytesText::from_escaped(content)))
                        .unwrap();
                }
            }
            _ => match &entry.event {
                Some(event) => {
                    writer.write_event(event).unwrap();
                }
                None => {}
            },
        }
    }

    println!("esi processing done.");

    Ok(writer.into_inner())
}
