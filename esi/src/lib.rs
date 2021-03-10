#[macro_use]
extern crate lazy_static;

use fancy_regex::Regex;

lazy_static! {
    // Self-enclosed tags, such as <esi:comment text="Just write some HTML instead"/>
    static ref EMPTY_TAG_REGEX: Regex = Regex::new(r"(?si)<esi:(?P<tag>[A-z]+)(?P<attributes>.*)/>").unwrap();

    // Tags with content, such as <esi:remove>test</esi:remove>
    static ref CONTENT_TAG_REGEX: Regex = Regex::new(r"(?si)<esi:(?P<tag>[A-z]+)(?P<attributes>.*)>(.*)</esi:(?P=tag)+>").unwrap();
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
    body = execute_empty_tags(body, client)?;
    body = execute_content_tags(body, client)?;

    println!("done.");

    Ok(body)
}

fn execute_empty_tags(body: String, client: &impl RequestHandler) -> Result<String, Error> {
    let element = EMPTY_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            println!("executing {}", element.as_str());

            let tag = EMPTY_TAG_REGEX.captures(&body).unwrap().unwrap().name("tag").unwrap().as_str();
            if tag == "include" {
                execute_empty_tags(body.replace(element.as_str(), "<span>include</span>"), client)
            } else {
                Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag)))
            }
        },
        None => Ok(body)
    }
}

fn execute_content_tags(body: String, client: &impl RequestHandler) -> Result<String, Error> {
    let element = CONTENT_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            println!("executing {}", element.as_str());

            let tag = CONTENT_TAG_REGEX.captures(&body).unwrap().unwrap().name("tag").unwrap().as_str();
            if tag == "remove" {
                execute_empty_tags(body.replace(element.as_str(), ""), client)
            } else {
                Err(Error::from_message(&format!("Unsupported tag: <esi:{}>", tag)))
            }
        },
        None => Ok(body)
    }}
