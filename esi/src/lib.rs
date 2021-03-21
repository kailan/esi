use std::{collections::HashMap, io::BufRead};
use quick_xml::{Reader, Writer, events::{BytesStart, Event}};

/// Contains information about errors encountered during ESI parsing or execution.
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn from_message(message: &str) -> Error {
        Error {
            message: String::from(message)
        }
    }
}

/// A request initiated by the ESI executor.
#[derive(Debug)]
pub struct Request {
    pub url: String
}

impl Request {
    fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string()
        }
    }
}

/// A response from the local `ExecutionContext` implementation.
/// Usually the result of a `Request`.
#[derive(Debug)]
pub struct Response {
    pub body: Vec<u8>,
    pub status_code: u16
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
    name: Vec<u8>, // "include"
    content: Option<String>, // "hello world"
    parameters: HashMap<Vec<u8>, Vec<u8>> // src = "/a.html"
}

pub struct TagEntry<'a> {
    event: Option<Event<'a>>,
    esi_tag: Option<Tag>
}

// This could be much cleaner but I'm not good enough at Rust for that
fn parse_attributes(bytes: BytesStart) -> Result<HashMap<Vec<u8>, Vec<u8>>, Error> {
    let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    for entry in bytes.attributes() {
        match entry {
            Ok(attr) => {
                match map.insert(attr.key.to_vec(), attr.value.to_vec()) {
                    Some(_) => return Err(Error::from_message("Attribute already defined")),
                    None => {}
                }
            }
            _ => {}
        }
    }

    Ok(map)
}

fn parse_tag_entries<'a>(body: impl BufRead) -> Result<Vec<TagEntry<'a>>, Error> {
    let mut reader = Reader::from_reader(body);

    reader
        .trim_markup_names_in_closing_tags(false)
        .check_end_names(false);

    let mut buf = Vec::new();

    let mut events: Vec<TagEntry> = Vec::new();

    let mut remove = false;

    // Parse tags and build events vec
    loop {
        buf.clear();
        match reader.read_event(&mut buf) {
            // Handle <esi:remove> tags
            Ok(Event::Start(elem)) if elem.starts_with(b"esi:remove") => {
                remove = true;
            },
            Ok(Event::End(elem)) if elem.starts_with(b"esi:remove") => {
                if !remove {
                    return Err(Error::from_message("Unexpected </esi:remove> closing tag"))
                }

                remove = false;
            },
            _ if remove => continue,

            // Parse empty ESI tags
            Ok(Event::Empty(elem)) if elem.name().starts_with(b"esi:") => {
                events.push(TagEntry {
                    event: None,
                    esi_tag: Some(Tag {
                        name: elem.name().to_vec(),
                        parameters: parse_attributes(elem)?,
                        content: None
                    })
                });
            },

            Ok(Event::Eof) => break,
            Ok(e) => events.push(TagEntry { event: Some(e.into_owned()), esi_tag: None }),
            _ => {}
        }
    }

    Ok(events)
}

/// Processes a given ESI response body and returns the transformed body after all ESI instructions
/// have been executed.
#[feature(option_unwrap_none)]
pub fn transform_esi_string(body: impl BufRead, client: &impl ExecutionContext) -> Result<Vec<u8>, Error> {
    let events = parse_tag_entries(body)?;
    // EXECUTE TAGS

    // Build output XML
    let mut writer = Writer::new(Vec::new());

    for entry in events {
        match entry.esi_tag {
            Some(tag) => {
                println!("tag received: {:?}", tag);
                // at this point, the content needs to be replaced
            },
            _ => writer.write_event(entry.event.unwrap()).unwrap()
        }
    }

    println!("done.");

    Ok(writer.into_inner())
}

/// Sends a request to the given `src`, optionally falling back to the `alt` if the first request is not successful.
fn send_request(src: &String, alt: Option<&String>, client: &impl ExecutionContext) -> Result<Response, Error> {
    match client.send_request(Request::from_url(src)) {
        Ok(resp) => Ok(resp),
        Err(err) => match alt {
            Some(alt) => match client.send_request(Request::from_url(alt)) {
                Ok(resp) => Ok(resp),
                Err(_) => Err(err)
            },
            None => Err(err)
        }
    }
}

// /// Recursively parses, executes, and replaces ESI tags (with no inner content) in the given body string.
// fn execute_empty_tags(body: String, client: &impl ExecutionContext) -> Result<String, Error> {
//     let element = EMPTY_TAG_REGEX.find(&body).unwrap_or_default();

//     match element {
//         Some(element) => {
//             let tag = Tag::from_captures(EMPTY_TAG_REGEX.captures(&body).unwrap().unwrap());

//             println!("Executing tag: {:?}", tag);

//             if tag.name == "include" {
//                 let src = match tag.parameters.get("src") {
//                     Some(src) => src,
//                     None => return Err(Error::from_message("No src parameter in <esi:include>"))
//                 };

//                 let alt = tag.parameters.get("alt");

//                 match send_request(src, alt, client) {
//                     Ok(resp) => {
//                         execute_empty_tags(body.replace(element.as_str(), &resp.body), client)
//                     },
//                     Err(err) => {
//                         match tag.parameters.get("onerror") {
//                             Some(onerror) => {
//                                 if onerror == "continue" {
//                                     println!("Failed to fetch {} but continued", src);
//                                     execute_empty_tags(body.replace(element.as_str(), ""), client)
//                                 } else {
//                                     Err(err)
//                                 }
//                             },
//                             _ => Err(err)
//                         }
//                     }
//                 }
//             } else if tag.name == "comment" {
//                 execute_empty_tags(body.replace(element.as_str(), ""), client)
//             } else {
//                 Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag.name)))
//             }
//         },
//         None => Ok(body)
//     }
// }

// /// Recursively parses, executes, and replaces ESI tags (with inner content) in the given body string.
// fn execute_content_tags(body: String, client: &impl ExecutionContext) -> Result<String, Error> {
//     let element = CONTENT_TAG_REGEX.find(&body).unwrap_or_default();

//     match element {
//         Some(element) => {
//             let tag = Tag::from_captures(CONTENT_TAG_REGEX.captures(&body).unwrap().unwrap());

//             println!("Executing tag: {:?}", tag);

//             if tag.name == "remove" {
//                 execute_content_tags(body.replace(element.as_str(), ""), client)
//             } else {
//                 Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag.name)))
//             }
//         },
//         None => Ok(body)
//     }}
