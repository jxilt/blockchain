A webserver that serves "Hello, World!" on the localhost root.

# Usage

## With Cargo

The webserver can be run using cargo. It takes a single command-line flag, `-p`, specifying the port to serve on. For example:

    cargo run server -p <port_number>

## With Docker

The webserver can be run using Docker, serving on port `10005`. For example:

    docker build -t jxilt/server .
    docker run --publish 10005:10005 --detach server:latest

## With Kubernetes

The webserver can be run using Kubernetes, serving on port `10005`. For example:

    docker push jxilt/server
    kubectl apply -f deployment.yaml
    minikube service server-entry