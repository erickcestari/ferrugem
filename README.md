# Ferrugem Load Balancer

A lightweight and efficient load balancer implemented in Rust.

## Overview

This project is a load balancer built using the Rust programming language and the Axum web framework. It is designed to be efficient and lightweight, capable of distributing incoming HTTP requests across multiple backend servers.

## Features

- Efficiently handles incoming HTTP requests and routes them to backend servers.
- Logs incoming requests and responses for easy debugging and monitoring.
- Configurable logging levels to control the amount of logged information.
- Utilizes the `reqwest` library for making HTTP requests to backend servers.
- Thread-safe, using `tokio::sync::Mutex` for state management.

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- Cargo (Rust package manager)
- Git

or

- Docker

### Usage with Docker

```sh
docker run -p 9999:9999 -v $(pwd)/ferrugem.toml:/usr/local/bin/ferrugem.toml erickcestari/ferrugem
```

### Installation

1. Clone the repository:

   ```sh
   git clone https://github.com/yourusername/rust-load-balancer.git
   cd rust-load-balancer
   ```

2. Build the project:

   ```sh
   cargo build --release
   ```

3. Run the load balancer:
   ```sh
   cargo run --release
   ```

### Configuration

The load balancer can be configured using a `ferrugem.toml` struct. Below is an example configuration:

```toml
version = 1
port = 9999
log_level = 'info'
algorithm = 'round-robin'

[[servers]]
name = "api1"
url = "https://jsonplaceholder.typicode.com"

[[servers]]
name = "api2"
url = "https://jsonplaceholder.typicode.com"
```

### Contributing

Contributions are welcome! Feel free to open issues or submit pull requests to improve the project.

### License

This project is licensed under the MIT License.
