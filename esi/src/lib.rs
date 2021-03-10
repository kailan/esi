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

pub fn transform_esi_string(body: String) -> Result<String, Error> {
    let empty = EMPTY_TAG_REGEX.captures_iter(&body);
    let content = CONTENT_TAG_REGEX.captures_iter(&body);

    for cap in empty {
        match cap {
            Ok(cap) => {
                let tag = cap.name("tag").unwrap().as_str();

                println!("tag with no content found: <esi:{}>", tag);
            }
            _ => {}
        }
    }

    for cap in content {
        match cap {
            Ok(cap) => {
                let tag = cap.name("tag").unwrap().as_str();

                println!("tag with content found: <esi:{}>", tag);
            }
            _ => {}
        }
    }

    println!("done.");

    Ok(body)
}
