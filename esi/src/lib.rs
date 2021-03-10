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
    fn from_message(message: String) -> Error {
        Error {
            message
        }
    }
}

pub fn transform_esi_string(mut body: String) -> Result<String, Error> {
    body = execute_empty_tags(body)?;
    body = execute_content_tags(body)?;

    println!("done.");

    Ok(body)
}

fn execute_empty_tags(body: String) -> Result<String, Error> {
    let element = EMPTY_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            println!("executing {}", element.as_str());

            let tag = EMPTY_TAG_REGEX.captures(&body).unwrap().unwrap().name("tag").unwrap().as_str();
            if tag == "include" {
                execute_empty_tags(body.replace(element.as_str(), "<span>include</span>"))
            } else {
                Err(Error::from_message(format!("Unsupported tag: <esi:{}>", tag)))
            }
        },
        None => Ok(body)
    }
}

fn execute_content_tags(body: String) -> Result<String, Error> {
    let element = CONTENT_TAG_REGEX.find(&body).unwrap_or_default();

    match element {
        Some(element) => {
            println!("executing {}", element.as_str());

            let tag = CONTENT_TAG_REGEX.captures(&body).unwrap().unwrap().name("tag").unwrap().as_str();
            if tag == "remove" {
                execute_empty_tags(body.replace(element.as_str(), ""))
            } else {
                Err(Error::from_message(format!("Unsupported tag: <esi:{}>", tag)))
            }
        },
        None => Ok(body)
    }}
