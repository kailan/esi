#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;

use fancy_regex::{Captures, Regex};

lazy_static! {
    // Self-enclosed tags, such as <esi:comment text="Just write some HTML instead"/>
    static ref EMPTY_TAG_REGEX: Regex = Regex::new(r"(?si)\s*<esi:(?P<tag>[A-z]+)(?P<parameters>.*?)/>\s*").unwrap();

    // Tags with content, such as <esi:remove>test</esi:remove>
    static ref CONTENT_TAG_REGEX: Regex = Regex::new(r"(?si)\s*<esi:(?P<tag>[A-z]+)(?P<parameters>.*?)>(?P<content>.+)</esi:(?P=tag)+>\s*").unwrap();

    // Parameters, e.g. data="test"
    static ref PARAMETER_REGEX: Regex = Regex::new(r#"\s*(.+?)="(.*?)""#).unwrap();
}

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

pub trait RequestHandler {
    /// Returns response body
    fn send_request(&self, url: &str) -> Result<String, Error>;
}

pub fn transform_esi_string(mut body: String, client: &impl RequestHandler) -> Result<String, Error> {
    body = execute_content_tags(body, client)?;
    body = execute_empty_tags(body, client)?;

    println!("done.");

    Ok(body)
}

#[derive(Debug)]
pub struct Tag {
    name: String, // "include"
    content: Option<String>, // "hello world"
    parameters: HashMap<String, String> // src = "/a.html"
}

impl Tag {
    fn empty_from_captures(cap: Captures) -> Tag {
        Tag {
            name: cap.name("tag").unwrap().as_str().to_string(),
            content: None,
            parameters: Tag::parse_parameters(cap.name("parameters").unwrap().as_str())
        }
    }

    fn with_content_from_captures(cap: Captures) -> Tag {
        Tag {
            name: cap.name("tag").unwrap().as_str().to_string(),
            content: Some(cap.name("content").unwrap().as_str().to_string()),
            parameters: Tag::parse_parameters(cap.name("parameters").unwrap().as_str())
        }
    }

    fn parse_parameters(input: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for cap in PARAMETER_REGEX.captures_iter(input) {
            match cap {
                Ok(cap) => {
                    map.insert(String::from(cap.get(1).unwrap().as_str()), String::from(cap.get(2).unwrap().as_str()));
                },
                _ => {}
            }
        }

        map
    }
}

fn execute_empty_tags(mut body: String, client: &impl RequestHandler) -> Result<String, Error> {
    let element = EMPTY_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            let tag = Tag::empty_from_captures(EMPTY_TAG_REGEX.captures(&body).unwrap().unwrap());

            println!("{:?}", tag);

            if tag.name == "include" {
                match tag.parameters.get("src") {
                    Some(src) => {
                        match client.send_request(src) {
                            Ok(resp) => {
                                body = body.replace(element.as_str(), &resp);
                                execute_empty_tags(body, client)
                            },
                            Err(err) => Err(err)
                        }
                    },
                    None => Err(Error::from_message("No src parameter in <esi:include>"))
                }
            } else if tag.name == "comment" {
                body = body.replace(element.as_str(), "");
                execute_empty_tags(body, client)
            } else {
                Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag.name)))
            }
        },
        None => Ok(body)
    }
}

fn execute_content_tags(mut body: String, client: &impl RequestHandler) -> Result<String, Error> {
    let element = CONTENT_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            let tag = Tag::with_content_from_captures(CONTENT_TAG_REGEX.captures(&body).unwrap().unwrap());

            println!("{:?}", tag);

            if tag.name == "remove" {
                body = body.replace(element.as_str(), "");
                execute_content_tags(body, client)
            } else {
                Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag.name)))
            }
        },
        None => Ok(body)
    }}
