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

//! A minimal implementation of consistent hashing
//!
//! Clients can use the `HashRing` struct to add consistent hashing to their
//! applications. `HashRing`'s API consists of three methods: `add`, `remove`,
//! and `get` for adding a node to the ring, removing a node from the ring, and
//! getting the node responsible for the provided key.
//!
//! original source: <https://github.com/jeromefroe/hashring-rs>
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
//! use hashring_coordinator::HashRing;
//! use std::net::IpAddr;
//! use std::str::FromStr;
//!
//! #[derive(Debug, Clone, Hash, PartialEq)]
//! struct Node {
//!     ip: IpAddr,
//! }
//! impl Node {
//!     fn new(ip: &str) -> Self {
//!         Node {
//!             ip: IpAddr::from_str(ip).unwrap(),
//!         }
//!     }
//! }
//! fn main() {
//!     // create a cluster with
//!     //      1 replication per key (each key is stored on 2 nodes)
//!     //      2 virtual nodes per real node
//!     let mut ring: HashRing<Node> = HashRing::new(1, 2);
//!
//!     // add some nodes to the cluster
//!     let nodes = vec![
//!         Node::new("127.0.0.1"),
//!         Node::new("127.0.0.2"),
//!         Node::new("127.0.0.3"),
//!     ];
//!
//!     ring.batch_add(nodes.clone());
//!
//!     // return list of nodes that store the key 'foo'
//!     // prints [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
//!     println!("{:?}", ring.get(&"foo"));
//!
//!     // return Vec<Replicas> containing hash ranges for each node of the cluster,
//!     // defining which keys to store or find where
//!     // the first node in each list can be considered the primary,
//!     // all following nodes are the replicas for the given hashrange
//!     // prints
//!     // [
//!     //      Replicas {
//!     //          hash_range: 15043474181722320698..=18446744073709551615,
//!     //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 0..=405901753359583262,                      
//!     //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 405901753359583263..=6291554157536183168,    
//!     //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 6291554157536183169..=7034287380452369431,   
//!     //          nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.2 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 7034287380452369432..=8537067609243575564,   
//!     //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 8537067609243575565..=11006066246803680578,  
//!     //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 11006066246803680579..=15043474181722320697,
//!      //         nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.1 }]
//!      //      }
//!     // ]
//!     println!("{:?}", ring.get_hash_ranges());
//!
//!     let mut ring2 = ring.clone();
//!     let new_node = Node::new("127.0.0.4");
//!     ring2.add(new_node.clone());
//!
//!     // return instructions (hashranges and nodes as given in struct Replicas)
//!     // to copy/move keys from ring1 to ring2
//!     // to replicate all keys to new_node that need to be stored there
//!     // prints
//!     //  [
//!     //      Replicas {
//!     //          hash_range: 11006066246803680579..=12253783648769497289,
//!     //          nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.1 }]
//!     //      },
//!     //      Replicas {
//!     //          hash_range: 7034287380452369432..=11006066246803680578,
//!     //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
//!     //      }
//!     //  ]
//!     println!("{:?}", ring2.find_sources(&new_node, &ring, &nodes));
//! }
//! ```

mod hashring;

pub use hashring::HashRing;
pub use hashring::coordinator::Replicas;
