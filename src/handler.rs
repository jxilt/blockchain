use std::io::{Read, Write};
use std::str::from_utf8;
use crate::persistence::{DbClient};

/// A handler for TCP streams.
pub trait Handler {
    // Handles incoming connections.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W);
}

/// A handler for HTTP requests.
pub struct HttpHandler<T: DbClient> {
    // Used to connect to the database.
    db_client: T
}

// TODO: Handle multiple routes (just two for now). To start, create static internal mapping of
//       routes, and serve them based on the path. Add tests (check that response contains one page
//       of HTML vs another; no need to check headers).
impl<T: DbClient> Handler for HttpHandler<T> {
    /// Checks the packet is properly formed, commits it to the database, and writes an ACK to the stream.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W) {
        let http_request = HttpHandler::<T>::read_http_request(reader);

        match http_request {
            Ok(_contents) => {
                let commit_result = self.db_client.commit("placeholder".to_string());

                match commit_result {
                    Ok(_) => HttpHandler::<T>::write_http_ok_response(writer),
                    Err(_) => HttpHandler::<T>::write_http_err_response(writer)
                }
            }
            Err(_) => HttpHandler::<T>::write_http_err_response(writer)
        };
    }
}

impl<T: DbClient> HttpHandler<T> {
    pub fn new(db_client: T) -> HttpHandler<T> {
        HttpHandler {
            db_client
        }
    }

    /// Extracts the method, URI and version from an incoming HTTP request.
    // TODO: Read headers, check post-header line, get message body.
    fn read_http_request<R: Read>(reader: R) -> Result<HttpRequest, String> {
        let mut tokens = Vec::<String>::new();
        let mut bytes = reader.bytes();

        let mut token = Vec::<u8>::new();
        loop {
            match bytes.next() {
                // We've reached the end of the current token.
                Some(Ok(b' ')) => {
                    let token_string = from_utf8(&token).expect("Request contained invalid UTF-8.").to_string();
                    tokens.push(token_string);
                    token.clear();
                }

                // We've reached the end of the line.
                Some(Ok(b'\r')) => {
                    let token_string = from_utf8(&token).expect("Request contained invalid UTF-8.").to_string();
                    tokens.push(token_string);

                    // We check that the next byte is a line-feed.
                    let maybe_line_feed = bytes.next();
                    return match maybe_line_feed {
                        // The start-line is correctly terminated by a CRLF.
                        Some(Ok(b'\n')) => {
                            if tokens.len() != 3 {
                                return Err("Malformed request line.".to_string());
                            }

                            let http_request = HttpRequest {
                                method: tokens[0].to_string(),
                                request_uri: tokens[1].to_string(),
                                http_version: tokens[2].to_string(),
                            };

                            Ok(http_request)
                        }
                        _ => Err("HTTP request start-line not terminated by CRLF.".to_string())
                    };
                }

                // We're mid-token.
                Some(Ok(byte)) => token.push(byte),

                // We failed to read the byte.
                Some(Err(_)) => return Err("Could not read bytes.".to_string()),

                // End of bytes.
                None => return Err("HTTP request start-line not terminated by CRLF.".to_string())
            }
        }
    }

    fn write_http_ok_response<W: Write>(mut writer: W) {
        let content = include_str!("hello_world.html");

        let header = format!("HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", content.len().to_string());

        writer.write(header.as_bytes()).expect("Failed to write HTTP response.");
        writer.write(content.as_bytes()).expect("Failed to write HTTP response.");
    }

    fn write_http_err_response<W: Write>(mut writer: W) {
        writer.write(b"HTTP/1.1 500 INTERNAL SERVER ERROR\r\n").expect("Failed to write HTTP response.");
    }
}

// TODO: Can I encapsulate this inside HttpHandler somehow?
pub struct HttpRequest {
    method: String,
    request_uri: String,
    http_version: String,
}

/// A dummy handler for testing.
pub struct DummyHandler;

impl Handler for DummyHandler {
    /// Reads the first byte. Enters an infinite loop if it reads the byte '#', which is useful for
    /// testing parallelism of the server. Otherwise, writes "DUMMY" to the stream.
    fn handle<R: Read, W: Write>(&self, reader: R, mut writer: W) {
        let mut bytes = reader.bytes();

        match bytes.next() {
            Some(Ok(b'#')) => loop { },
            // TODO: Handle the error case separately?
            _ => {
                writer.write(b"DUMMY\n").expect("Writing failed.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, BufWriter};
    use std::str::from_utf8;
    use crate::persistence::{DummyDbClient};
    use crate::handler::{Handler, HttpHandler};

    fn handle(request: String) -> String {
        let mut response = Vec::<u8>::new();

        let db_client = DummyDbClient {};
        let handler = HttpHandler::new(db_client);

        let reader = BufReader::new(request.as_bytes());
        let writer = BufWriter::new(&mut response);

        handler.handle(reader, writer);

        return from_utf8(&response).expect("Response was invalid UTF-8.").to_string();
    }

    #[test]
    fn handler_accepts_valid_http_requests_and_returns_expected_response() {
        let valid_request = "GET / HTTP/1.1\r\n";
        let response = handle(valid_request.to_string());

        let expected_body = include_str!("hello_world.html");

        let expected_body_length = expected_body.len();
        let expected_headers = format!("HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", expected_body_length.to_string());
        let expected_response = expected_headers + expected_body;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn handler_rejects_invalid_http_requests() {
        let invalid_requests = [
            "\r\n", // Too few items.
            "GET\r\n", // Too few items.
            "GET /\r\n", // Too few items.
            "GET / HTTP/1.1 EXTRA\r\n", // Too many items.
            "GET / HTTP/1.1", // Missing CRLF.
            "GET / HTTP/1.1 EXTRA\r", // Missing LF.
            "GET / HTTP/1.1\n", // Missing CR.
            "GET / HTTP/1.1 EXTRA\n\r" // CR and LF in wrong order.
        ];

        for request in &invalid_requests {
            let response = handle(request.to_string());

            assert_eq!(response, "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n");
        }
    }
}