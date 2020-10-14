use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::str::{from_utf8};

use crate::servererror::{Result, ServerError};
use crate::persistence::DbClient;

const ERROR_PAGE_404: &str = "./src/html/404.html";
const ERROR_PAGE_500: &str = "./src/html/500.html";

/// A handler for TCP streams.
pub trait Handler {
    // Handles incoming connections.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W) -> Result<()>;
}

/// A handler for HTTP requests.
pub struct HttpHandler<T: DbClient> {
    // Used to connect to the database.
    db_client: T,
    // Used to store the server's routes.
    routes: HashMap<String, String>
}

impl<T: DbClient> Handler for HttpHandler<T> {
    /// Reads the HTTP request, handles it and writes an HTTP response.
    fn handle<R: Read, W: Write>(&self, reader: R, writer: W) -> Result<()> {
        let http_request = HttpHandler::<T>::read_http_request(reader);

        return match http_request {
            Err(_e) => HttpHandler::<T>::write_http_500_response(writer),
            Ok(http_request) => {
                let maybe_file_path = self.routes.get(&http_request.request_uri);

                match maybe_file_path {
                    None => HttpHandler::<T>::write_http_404_response(writer),
                    Some(file_path) => HttpHandler::<T>::write_http_ok_response(writer, file_path)
                }
            }
        };
    }
}

impl <T: DbClient> HttpHandler<T> {
    pub fn new(db_client: T, routes: HashMap<String, String>) -> HttpHandler<T> {
        HttpHandler {
            db_client,
            routes
        }
    }

    /// Extracts the method, URI and version from an incoming HTTP request.
    // TODO: Read headers, check post-header line, get message body.
    fn read_http_request<R: Read>(reader: R) -> Result<HttpRequest> {
        let mut incoming_bytes = reader.bytes();
        let mut current_token = Vec::<u8>::new();
        let mut tokens = Vec::<String>::new();

        loop {
            let current_byte = incoming_bytes.next()
                // We've reached the end of the bytes without encountering a CRLF.
                .ok_or(ServerError { message: "HTTP request ended without CRLF.".to_string() })?
                // We've failed to read the byte.
                ?;

            match current_byte {
                // We've reached the end of the current token.
                b' ' => {
                    let token_string = from_utf8(&current_token)?;
                    tokens.push(token_string.to_string());
                    current_token.clear();
                }

                // We've reached the end of the line.
                b'\r' => {
                    let token_string = from_utf8(&current_token)?;
                    tokens.push(token_string.to_string());

                    // We check that the next byte is a line-feed.
                    let maybe_line_feed = incoming_bytes.next()
                        // There is no next byte.
                        .ok_or(ServerError { message: "HTTP request start-line not terminated by CRLF.".to_string() })?
                        // We've failed to read the byte.
                        ?;

                    return match maybe_line_feed {
                        // The start-line is correctly terminated by a CRLF.
                        b'\n' => {
                            if tokens.len() != 3 {
                                return Err(ServerError { message: "Request line does not have three tokens.".to_string() })
                            }

                            Ok(HttpRequest {
                                method: tokens[0].to_string(),
                                request_uri: tokens[1].to_string(),
                                http_version: tokens[2].to_string(),
                            })
                        }
                        _ => Err(ServerError { message: "HTTP request start-line not terminated by LF.".to_string() })
                    };
                }

                // We're mid-token.
                any_other_byte => current_token.push(any_other_byte),
            }
        }
    }

    /// Writes a valid HTTP response.
    fn write_http_ok_response<W: Write>(writer: W, file_path: &str) -> Result<()> {
        return HttpHandler::<T>::write_http_response(writer, "200 OK", file_path);
    }

    /// Writes a 500 HTTP response.
    fn write_http_500_response<W: Write>(writer: W) -> Result<()> {
        return HttpHandler::<T>::write_http_response(writer, "500 INTERNAL SERVER ERROR", ERROR_PAGE_500);
    }

    /// Writes a 404 HTTP response.
    fn write_http_404_response<W: Write>(writer: W) -> Result<()> {
        return HttpHandler::<T>::write_http_response(writer, "404 NOT FOUND", ERROR_PAGE_404);
    }

    /// Writes an HTTP response for a given status code and page.
    fn write_http_response<W: Write>(mut writer: W, status_code: &str, file_path: &str) -> Result<()> {
        let html = fs::read_to_string(file_path)?;

        let headers = format!("HTTP/1.1 {}\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/html\r\n\
            Connection: Closed\r\n\r\n", status_code, html.len().to_string());

        writer.write((headers + &html).as_bytes())?;

        return Ok(());
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
    fn handle<R: Read, W: Write>(&self, reader: R, mut writer: W) -> Result<()> {
        let byte = reader.bytes().next()
            // There were no bytes to read.
            .ok_or(ServerError { message: "Nothing to read from stream.".to_string() })?
            // We've failed to read the byte.
            ?;

        match byte {
            b'#' => loop { },
            _ => {
                writer.write(b"DUMMY\n")?;
            }
        }

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufReader, BufWriter};
    use std::str::from_utf8;

    use crate::handler::{Handler, HttpHandler};
    use crate::persistence::DummyDbClient;

    const ERROR_PAGE_404: &str = "./src/html/404.html";
    const ERROR_PAGE_500: &str = "./src/html/500.html";

    fn handle(request: String) -> String {
        let mut response = Vec::<u8>::new();

        let db_client = DummyDbClient {};
        let routes = [
            ("/".to_string(), "./src/html/hello_world.html".to_string()),
            ("/2".to_string(), "./src/html/hello_world_2.html".to_string())
        ].iter().cloned().collect();
        let handler = HttpHandler::new(db_client, routes);

        let reader = BufReader::new(request.as_bytes());
        let writer = BufWriter::new(&mut response);

        handler.handle(reader, writer).unwrap();

        return from_utf8(&response).unwrap().to_string();
    }

    #[test]
    fn handler_accepts_valid_http_requests_and_returns_expected_response() {
        let valid_requests_and_file_paths = [
            ("GET / HTTP/1.1\r\n", "./src/html/hello_world.html"),
            ("GET /2 HTTP/1.1\r\n", "./src/html/hello_world_2.html")
        ];

        for (valid_request, file_path) in valid_requests_and_file_paths.iter() {
            let response = handle(valid_request.to_string());

            let expected_body = fs::read_to_string(file_path).unwrap();
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

        let expected_body = fs::read_to_string(ERROR_PAGE_500).unwrap();
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

        let expected_body = fs::read_to_string(ERROR_PAGE_404).unwrap();
        let expected_headers = format!("HTTP/1.1 404 NOT FOUND\r\n\
                Content-Length: {}\r\n\
                Content-Type: text/html\r\n\
                Connection: Closed\r\n\r\n", expected_body.len().to_string());
        let expected_response = expected_headers + &expected_body;

        assert_eq!(response, expected_response);
    }
}