use std::io::{Read, Write};
use std::str::from_utf8;
use crate::persistence::{DbClient};
use std::collections::HashMap;
use std::fs;

const ERROR_PAGE_404: &str = "./src/404.html";
const ERROR_PAGE_500: &str = "./src/500.html";

/// A handler for TCP streams.
pub trait Handler {
    // Handles incoming connections.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W);
}

/// A handler for HTTP requests.
pub struct HttpHandler<T: DbClient> {
    // Used to connect to the database.
    db_client: T,
    // Used to store the server's routes.
    routes: HashMap<String, String>
}

impl<T: DbClient> Handler for HttpHandler<T> {
    /// Checks the packet is properly formed, commits it to the database, and writes an ACK to the stream.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W) {
        let http_request = HttpHandler::<T>::read_http_request(reader);

        match http_request {
            Err(_e) => HttpHandler::<T>::write_http_500_response(writer),
            Ok(http_request) => {
                let maybe_file_path = self.routes.get(&http_request.request_uri);

                match maybe_file_path {
                    None => HttpHandler::<T>::write_http_404_response(writer),
                    Some(file_path) => {
                        let maybe_html = fs::read_to_string(file_path);

                        match maybe_html {
                            Err(_e) => HttpHandler::<T>::write_http_500_response(writer),
                            Ok(html) =>  HttpHandler::<T>::write_http_ok_response(writer, html.to_string())
                        }
                    }
                }
            }
        };
    }
}

impl<T: DbClient> HttpHandler<T> {
    pub fn new(db_client: T, routes: HashMap<String, String>) -> HttpHandler<T> {
        HttpHandler {
            db_client,
            routes
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
                    let token_string = from_utf8(&token);
                    match token_string {
                        Err(_e) => return Err("Request contained invalid UTF-8.".to_string()),
                        Ok(valid_token_string) => {
                            tokens.push(valid_token_string.to_string());
                            token.clear();
                        }
                    }
                }

                // We've reached the end of the line.
                Some(Ok(b'\r')) => {
                    let token_string = from_utf8(&token);
                    match token_string {
                        Err(_e) => return Err("Request contained invalid UTF-8.".to_string()),
                        Ok(valid_token_string) => {
                            tokens.push(valid_token_string.to_string());
                        }
                    }

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
                Some(Err(_e)) => return Err("Could not read bytes.".to_string()),

                // We've reached the end of the bytes without encountering a CRLF.
                None => return Err("HTTP request start-line not terminated by CRLF.".to_string())
            }
        }
    }

    /// Writes a valid HTTP response.
    fn write_http_ok_response<W: Write>(mut writer: W, html: String) {
        let header = format!("HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", html.len().to_string());

        writer.write((header + &html).as_bytes()).expect("Failed to write HTTP response.");
    }

    /// Writes a 500 HTTP response.
    fn write_http_500_response<W: Write>(mut writer: W) {
        let html = fs::read_to_string(ERROR_PAGE_500).expect("Failed to find error page.");

        let header = format!("HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", html.len().to_string());

        writer.write((header + &html).as_bytes()).expect("Failed to write HTTP response.");
    }

    /// Writes a 404 HTTP response.
    fn write_http_404_response<W: Write>(mut writer: W) {
        let html = fs::read_to_string(ERROR_PAGE_404).expect("Failed to find error page.");

        let header = format!("HTTP/1.1 404 NOT FOUND\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", html.len().to_string());

        writer.write((header + &html).as_bytes()).expect("Failed to write HTTP response.");
    }
}

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
            _ => {
                writer.write(b"DUMMY\n").expect("Writing failed.");
            }
        }

        ()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, BufWriter};
    use std::str::from_utf8;
    use crate::persistence::{DummyDbClient};
    use crate::handler::{Handler, HttpHandler};
    use std::collections::HashMap;
    use std::fs;

    const ERROR_PAGE_404: &str = "./src/404.html";
    const ERROR_PAGE_500: &str = "./src/500.html";

    fn handle(request: String) -> String {
        let mut response = Vec::<u8>::new();

        let db_client = DummyDbClient {};
        let mut routes = HashMap::new();
        routes.insert("/".to_string(), "./src/hello_world.html".to_string());
        routes.insert("/2".to_string(), "./src/hello_world_2.html".to_string());
        let handler = HttpHandler::new(db_client, routes);

        let reader = BufReader::new(request.as_bytes());
        let writer = BufWriter::new(&mut response);

        handler.handle(reader, writer);

        return from_utf8(&response).expect("Response was invalid UTF-8.").to_string();
    }

    #[test]
    fn handler_accepts_valid_http_requests_and_returns_expected_response() {
        let valid_requests_and_body_paths = [
            ("GET / HTTP/1.1\r\n", "./src/hello_world.html"),
            ("GET /2 HTTP/1.1\r\n", "./src/hello_world_2.html")
        ];

        for (valid_request, body_path) in valid_requests_and_body_paths.iter() {
            let response = handle(valid_request.to_string());

            let expected_body = fs::read_to_string(body_path).expect("Could not find file.");
            let expected_headers = format!("HTTP/1.1 200 OK\r\n\
                Content-Length: {}\r\n\
                Content-Type: text/html\r\n\
                Connection: Closed\r\n\r\n", expected_body.len().to_string());
            let expected_response = expected_headers + &expected_body;

            assert_eq!(response, expected_response);
        }
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
            "GET / HTTP/1.1 EXTRA\n\r", // CR and LF in wrong order.
            // TODO: Test of invalid UTF-8.
        ];

        let expected_body = fs::read_to_string(ERROR_PAGE_500).expect("Failed to find error page.");
        let expected_headers = format!("HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\
                Content-Length: {}\r\n\
                Content-Type: text/html\r\n\
                Connection: Closed\r\n\r\n", expected_body.len().to_string());
        let expected_response = expected_headers + &expected_body;

        for request in invalid_requests.iter() {
            let response = handle(request.to_string());

            assert_eq!(response, expected_response);
        }
    }

    #[test]
    fn handler_rejects_unknown_routes() {
        let valid_request = "GET /unknown_route HTTP/1.1\r\n";
        let response = handle(valid_request.to_string());

        let expected_body = fs::read_to_string(ERROR_PAGE_404).expect("Failed to find error page.");
        let expected_headers = format!("HTTP/1.1 404 NOT FOUND\r\n\
                Content-Length: {}\r\n\
                Content-Type: text/html\r\n\
                Connection: Closed\r\n\r\n", expected_body.len().to_string());
        let expected_response = expected_headers + &expected_body;

        assert_eq!(response, expected_response);
    }
}