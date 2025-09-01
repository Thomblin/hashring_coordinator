// MIT License

// Copyright (c) 2016 Jerome Froelich

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A minimal implementation of consistent hashing as described in [Consistent
//! Hashing and Random Trees: Distributed Caching Protocols for Relieving Hot
//! Spots on the World Wide Web](https://www.akamai.com/es/es/multimedia/documents/technical-publication/consistent-hashing-and-random-trees-distributed-caching-protocols-for-relieving-hot-spots-on-the-world-wide-web-technical-publication.pdf).
//! Clients can use the `HashRing` struct to add consistent hashing to their
//! applications. `HashRing`'s API consists of three methods: `add`, `remove`,
//! and `get` for adding a node to the ring, removing a node from the ring, and
//! getting the node responsible for the provided key.
//!
//! original source: https://github.com/jeromefroe/hashring-rs
//!
//! ## Example
//!
//! Below is a simple example of how an application might use `HashRing` to make
//! use of consistent hashing. Since `HashRing` exposes only a minimal API clients
//! can build other abstractions, such as virtual nodes, on top of it. The example
//! below shows one potential implementation of virtual nodes on top of `HashRing`
//!
//! ``` rust,no_run
//! extern crate hashring_coordinator;
//!
//! use std::net::{IpAddr, SocketAddr};
//! use std::str::FromStr;
//!
//! use hashring_coordinator::HashRing;
//!
//! #[derive(Debug, Copy, Clone, Hash, PartialEq)]
//! struct Node {
//!     ip: IpAddr,
//! }
//!
//! impl Node {
//!     fn new(ip: &str) -> Self {
//!         Node {
//!             ip: IpAddr::from_str(&ip).unwrap(),
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let mut ring: HashRing<Node> = HashRing::new(2, 200);
//!
//!     let mut nodes = vec![];
//!     nodes.push(Node::new("127.0.0.1"));
//!     nodes.push(Node::new("127.0.0.2"));
//!     nodes.push(Node::new("127.0.0.3"));
//!
//!     for node in nodes {
//!         ring.add(node);
//!     }
//!
//!     println!("{:?}", ring.get(&"foo"));
//!     println!("{:?}", ring.get(&"bar"));
//!     println!("{:?}", ring.get(&"baz"));
//! }
//! ```

mod hashring;

pub use hashring::HashRing;
