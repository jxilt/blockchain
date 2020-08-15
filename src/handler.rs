use std::io::{Read, BufReader, BufWriter, Write};
use std::str::from_utf8;
use std::net::TcpStream;
use crate::persistence::{DbClient};

/// A handler for TCP streams.
pub trait Handler {
    // Handles incoming connections.
    fn handle(&self, stream: TcpStream);
}

/// A dummy handler for testing.
pub struct DummyHandler {}

impl Handler for DummyHandler {
    /// Writes "DUMMY" to every stream.
    fn handle(&self, stream: TcpStream) {
        let mut writer = BufWriter::new(&stream);
        writer.write(b"DUMMY\n").expect("Writing failed.");
    }
}

/// A handler for HTTP requests.
pub struct HttpHandler <T: DbClient> {
    // Used to connect to the database.
    db_client: T
}

impl <T: DbClient> Handler for HttpHandler<T> {
    /// Checks the packet is properly formed, commits it to the database, and writes an ACK to the stream.
    fn handle(&self, stream: TcpStream) {
        let reader = BufReader::new(&stream);
        let http_request = HttpHandler::<T>::read_http_request(reader);

        let writer = BufWriter::new(&stream);
        match http_request {
            Ok(contents) => {
                let commit_result = self.db_client.commit("placeholder".to_string());

                match commit_result {
                    Ok(_) => HttpHandler::<T>::write_http_ok_response(writer),
                    Err(_) => HttpHandler::<T>::write_http_err_response(writer)
                }
            },
            Err(_) => HttpHandler::<T>::write_http_err_response(writer)
        };
    }
}

impl <T: DbClient> HttpHandler<T> {
    pub fn new (db_client: T) -> HttpHandler<T> {
        HttpHandler {
            db_client
        }
    }

    /// Extracts the method, URI and version from an incoming HTTP request.
    // TODO: Read headers.
    // TODO: Check post-header line.
    // TODO: Get message body.
    fn read_http_request <R: Read> (reader: R) -> Result<HttpRequest, String> {
        let mut tokens = Vec::<String>::new();
        let mut bytes = reader.bytes();

        let mut token = Vec::<u8>::new();
        loop {
            match bytes.next() {
                // We've reached the end of the current token.
                Some(Ok(b' ')) => {
                    let token_string = from_utf8(&token).expect("Token was invalid UTF-8.").to_string();
                    tokens.push(token_string);
                    token.clear();
                },

                // We've reached the end of the line.
                Some(Ok(b'\r')) => {
                    let token_string = from_utf8(&token).expect("Token was invalid UTF-8.").to_string();
                    tokens.push(token_string);

                    // We check that the next byte is a line-feed.
                    let maybe_line_feed = bytes.next();
                    match maybe_line_feed {
                        // The start-line is correctly terminated by a CRLF.
                        Some(Ok(b'\n')) => {
                            if tokens.len() != 3 {
                                return Err("Malformed request line.".to_string());
                            }
                    
                            let http_request = HttpRequest {
                                method: tokens[0].to_string(),
                                request_uri: tokens[1].to_string(),
                                http_version: tokens[2].to_string()
                            };
                    
                            return Ok(http_request);
                        },
                        _ => return Err("HTTP request start-line not terminated by CRLF.".to_string())
                    }
                },

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
        writer.write(b"HTTP/1.1 200 OK\r\n").expect("Failed to write HTTP response.");
    }

    fn write_http_err_response<W: Write>(mut writer: W) {
        writer.write(b"HTTP/1.1 500 INTERNAL SERVER ERROR\r\n").expect("Failed to write HTTP response.");
    }
}

pub struct HttpRequest {
    method: String,
    request_uri: String,
    http_version: String
}

// TODO: Add tests.