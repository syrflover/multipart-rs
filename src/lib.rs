use bytes::Bytes;
use http::{header::HeaderName, HeaderMap, HeaderValue};

/// from content_type
pub fn parse_boundary<'a>(header_value: &'a HeaderValue) -> Option<&'a [u8]> {
    let bytes = header_value.as_bytes();
    let pos = memchr::memmem::find(bytes, b"boundary=")?;

    Some(&bytes[(pos + 9)..])
}

pub struct Multipart<'a> {
    boundary: Vec<u8>,
    bytes: &'a Bytes,
    pos: usize,
    end: usize,
}

impl<'a> Multipart<'a> {
    ///
    /// returns None
    /// - can't find boundary
    pub fn new(boundary: impl AsRef<[u8]>, bytes: &'a Bytes) -> Multipart<'a> {
        let boundary = [b"--", boundary.as_ref()]
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<u8>>();

        // twoway
        let pos = memchr::memmem::find(&bytes, &boundary).unwrap() + boundary.len(); // ignore first boundary
        let end = memchr::memmem::rfind(&bytes, &boundary).unwrap(); // end boundary position

        // println!("pos = {pos}");
        // println!("end = {end}");

        Self {
            boundary,
            bytes,
            pos,
            end,
        }
    }
}

impl<'a> Iterator for Multipart<'a> {
    type Item = (HeaderMap, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let end = self.pos + memchr::memmem::find(&self.bytes[self.pos..], &self.boundary)?;

        // println!("start = {}", self.pos);
        // println!("end = {end}");

        let r = &self.bytes[self.pos..end];

        // println!("{}", String::from_utf8(r.to_vec()).unwrap());

        // println!("{r:?}");

        if r.is_empty() || self.pos >= self.end {
            // println!("start = {}", self.pos);
            // println!("end = {end}");

            return None;
        }

        self.pos = end + self.boundary.len();

        let mut headers = HeaderMap::new();

        let crlf_pos = memchr::memmem::find(r, b"\r\n\r\n").unwrap();

        if let Ok(x) = String::from_utf8(r[..crlf_pos].to_vec()) {
            let lines = x.trim().lines();
            for x in lines {
                let mut it = x.splitn(2, ':');

                let header = it
                    .next()
                    .and_then(|name| {
                        it.next()
                            .map(|value| (name.to_owned(), value.trim().to_owned()))
                    })
                    .and_then(|(name, value)| {
                        Some((
                            HeaderName::from_bytes(name.as_bytes()).ok()?,
                            HeaderValue::from_str(&value).ok()?,
                        ))
                    });

                if let Some((name, value)) = header {
                    headers.insert(name, value);
                }
            }
        }

        // crlf_pos + "\r\n\r\n".len()
        let body = r[(crlf_pos + 4)..].to_vec();

        // println!("headers = {}", String::from_utf8(headers.to_vec()).unwrap());
        // println!("body = {}", String::from_utf8(body.to_vec()).unwrap());

        Some((headers, body))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http::HeaderValue;

    use crate::parse_boundary;

    use super::Multipart;

    #[test]
    fn test_multipart() {
        let boundary = "abhjdahkdhfsikldhjfliawefrkhkahskda";

        let body = format!(
            r#"--{boundary}
Content-Type: application/http
Content-ID: response-

HTTP/1.1 200 OK
Content-Type: application/json; charset=UTF-8
Vary: Origin
Vary: X-Origin
Vary: Referer

{{
"name": "projects/35006771263/messages/0:1570471792141125%43c11b7043c11b70"
}}

--{boundary}
Content-Type: application/http
Content-ID: response-

HTTP/1.1 400 BAD REQUEST
Content-Type: application/json; charset=UTF-8
Vary: Origin
Vary: X-Origin
Vary: Referer

{{
"error": {{
    "code": 400,
    "message": "The registration token is not a valid FCM registration token",
    "status": "INVALID_ARGUMENT"
  }}
}}

--{boundary}
Content-Type: application/http
Content-ID: response-

HTTP/1.1 200 OK
Content-Type: application/json; charset=UTF-8
Vary: Origin
Vary: X-Origin
Vary: Referer

{{adnsdkjasdh
"name": "projects/35006771263/messages/0:1570471792141696%43c11b7043c11b70"
}}

--{boundary}--"#
        )
        .replace('\n', "\r\n")
        .replace("adnsdkjasdh", "\r\r\r\r\r\r\r\r");

        let content_type =
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}")).unwrap();
        let bytes = Bytes::from_iter(body.bytes());

        let multipart = Multipart::new(parse_boundary(&content_type).unwrap(), &bytes);

        // let a = String::from_utf8(multipart.next().unwrap()).unwrap();

        // println!("{a}");

        for (headers, a) in multipart {
            let content_type = headers.get("Content-Type").unwrap().to_str().unwrap();
            let content_id = headers.get("Content-ID").unwrap().to_str().unwrap();

            println!("Content-Type: {content_type}");
            println!("Content-ID: {content_id}");
            println!();

            let a = String::from_utf8(a).unwrap();

            let r = a.contains('\r');

            assert!(r);

            println!("{a}");

            // println!("{}", a.replace('\r', "\\r"));

            println!("------------------");
        }
    }
}
