use quick_xml::{
    events::{BytesStart, BytesText, Event},
    Reader, Writer,
};
use std::{collections::HashMap, io::BufRead};

pub struct Configuration {
    namespace: String
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            namespace: String::from("esi")
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

/// Contains information about errors encountered during ESI parsing or execution.
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn from_message(message: &str) -> Error {
        Error {
            message: String::from(message),
        }
    }
}

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
    fn send_request(&self, req: Request) -> Result<Response, Error>;
}

/// Representation of an ESI tag from a source response.
#[derive(Debug)]
pub struct Tag {
    name: Vec<u8>,                         // "include"
    content: Option<String>,               // "hello world"
    parameters: HashMap<Vec<u8>, Vec<u8>>, // src = "/a.html"
}

impl Tag {
    fn get_param(&self, key: &str) -> Option<String> {
        match self.parameters.get(key.as_bytes()) {
            Some(value) => Some(String::from_utf8(value.to_owned()).unwrap()),
            None => None,
        }
    }
}

pub struct TagEntry<'a> {
    event: Option<Event<'a>>,
    esi_tag: Option<Tag>,
}

// This could be much cleaner but I'm not good enough at Rust for that
fn parse_attributes(bytes: BytesStart) -> Result<HashMap<Vec<u8>, Vec<u8>>, Error> {
    let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    for entry in bytes.attributes() {
        match entry {
            Ok(attr) => match map.insert(attr.key.to_vec(), attr.value.to_vec()) {
                Some(_) => return Err(Error::from_message("Attribute already defined")),
                None => {}
            },
            _ => {}
        }
    }

    Ok(map)
}

fn parse_tag_entries<'a>(body: impl BufRead, configuration: &Configuration) -> Result<Vec<TagEntry<'a>>, Error> {
    let mut reader = Reader::from_reader(body);
    let mut buf = Vec::new();

    let mut events: Vec<TagEntry> = Vec::new();
    let mut remove = false;

    let esi_remove = format!("{}:remove", configuration.namespace);
    let esi_empty = format!("{}:", configuration.namespace);

    // Parse tags and build events vec
    loop {
        buf.clear();
        match reader.read_event(&mut buf) {
            // Handle <esi:remove> tags
            Ok(Event::Start(elem)) if elem.starts_with(esi_remove.as_bytes()) => {
                remove = true;
            }
            Ok(Event::End(elem)) if elem.starts_with(esi_remove.as_bytes()) => {
                if !remove {
                    let message = format!("Unexpected </{}:remove> closing tag", configuration.namespace);
                    return Err(Error::from_message(&message));
                }

                remove = false;
            }
            _ if remove => continue,

            // Parse empty ESI tags
            Ok(Event::Empty(elem)) if elem.name().starts_with(esi_empty.as_bytes()) => {
                events.push(TagEntry {
                    event: None,
                    esi_tag: Some(Tag {
                        name: elem.name().to_vec(),
                        parameters: parse_attributes(elem)?,
                        content: None,
                    }),
                });
            }

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
fn execute_tag_entries<'a>(
    entries: &'a Vec<TagEntry>,
    client: &impl ExecutionContext,
    configuration: &Configuration
) -> Result<HashMap<usize, Vec<u8>>, Error> {
    let mut map = HashMap::new();

    let esi_include = format!("{}:include", configuration.namespace);

    for (index, entry) in entries.iter().enumerate() {
        match &entry.esi_tag {
            Some(tag) => {
                if tag.name == esi_include.as_bytes() {
                    let src = match tag.get_param("src") {
                        Some(src) => src,
                        None => {
                            let message = format!("No src parameter in <{}:include>", configuration.namespace);
                            return Err(Error::from_message(&message))
                        }
                    };

                    let alt = tag.get_param("alt");

                    match send_request(&src, alt.as_ref(), client) {
                        Ok(resp) => match map.insert(index, resp.body) {
                            _ => {}
                        },
                        Err(err) => match tag.get_param("onerror") {
                            Some(onerror) => {
                                if onerror == "continue" {
                                    println!("Failed to fetch {} but continued", src);
                                    match map.insert(index, vec![]) {
                                        _ => {}
                                    }
                                } else {
                                    return Err(err);
                                }
                            }
                            _ => return Err(err),
                        },
                    }
                }
            }
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
    configuration: &Configuration
) -> Result<Vec<u8>, Error> {
    // Parse tags
    let events = parse_tag_entries(body, configuration)?;

    // Execute tags
    let results = execute_tag_entries(&events, client, configuration)?;

    // Build output XML
    let mut writer = Writer::new(Vec::new());

    for (index, entry) in events.iter().enumerate() {
        match &entry.esi_tag {
            Some(_tag) => {
                match results.get(&index) {
                    Some(content) => {
                        writer
                            .write_event(Event::Text(BytesText::from_escaped(content)))
                            .unwrap();
                    }
                    None => {}
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

/// Sends a request to the given `src`, optionally falling back to the `alt` if the first request is not successful.
fn send_request(
    src: &String,
    alt: Option<&String>,
    client: &impl ExecutionContext,
) -> Result<Response, Error> {
    match client.send_request(Request::from_url(src)) {
        Ok(resp) => Ok(resp),
        Err(err) => match alt {
            Some(alt) => match client.send_request(Request::from_url(alt)) {
                Ok(resp) => Ok(resp),
                Err(_) => Err(err),
            },
            None => Err(err),
        },
    }
}
