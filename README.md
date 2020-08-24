A webserver that serves "Hello, World!" on the root.

# Usage

## With Cargo

The webserver takes a single command-line flag, `-p`, specifying the port to serve on. For example:

    cargo run blockchain -p <port_number>

## With Docker

The webserver can run using Docker, serving on port `10005`. For example:

    docker build .
    docker run --publish 10005:10005 --detach <image_name>