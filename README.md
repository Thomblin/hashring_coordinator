A minimal implementation of consistent hashing as described in [Consistent
Hashing and Random Trees: Distributed Caching Protocols for Relieving Hot
Spots on the World Wide Web](https://www.akamai.com/es/es/multimedia/documents/technical-publication/istent-hashing-and-random-trees-distributed-caching-protocols-for-relieving-hot-spots-on-the-world-wide-web-technical-publication.pdf).

Clients can use the `HashRing` struct to add consistent hashing to their
applications. 

- You can add and remove nodes to a HashRing.
- Find all nodes that (should) store a given key.
- Return all hash ranges within the HashRing to easily detect nodes and their responsibilities (containing replica nodes as well)
- Compare two HashRing clusters to receive replication instructions between both clusters (for each node, list hash ranges and target nodes to find keys that need to be replicated)

This implemementation is based on the original source: <https://github.com/jeromefroe/hashring-rs>

## Example

Below is a simple example of how an application might use `HashRing` to make
use of consistent hashing. Since `HashRing` exposes only a minimal API clients
can build other abstractions, such as virtual nodes, on top of it. The example
below shows one potential implementation of virtual nodes on top of `HashRing`

```rust
extern crate hashring_coordinator;
use hashring_coordinator::HashRing;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Hash, PartialEq)]
struct Node {
    ip: IpAddr,
}

impl Node {
    fn new(ip: &str) -> Self {
        Node {
            ip: IpAddr::from_str(ip).unwrap(),
        }
    }
}

fn main() {
    // create a cluster with
    //      1 replication per key (each key is stored on 2 nodes)
    //      2 virtual nodes per real node
    let mut ring: HashRing<Node> = HashRing::new(1, 2);
    
    // add some nodes to the cluster
    let nodes = vec![
        Node::new("127.0.0.1"),
        Node::new("127.0.0.2"),
        Node::new("127.0.0.3"),
    ];
    ring.batch_add(nodes.clone());

    // return list of nodes that store the key 'foo'
    // prints [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
    println!("{:?}", ring.get(&"foo"));

    // return Vec<Replicas> containing hash ranges for each node of the cluster,
    // defining which keys to store or find where
    // the first node in each list can be considered the primary,
    // all following nodes are the replicas for the given hashrange
    // prints
    // [
    //      Replicas {
    //          hash_range: 15043474181722320698..=18446744073709551615,
    //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
    //      },
    //      Replicas {
    //          hash_range: 0..=405901753359583262,                      
    //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
    //      },
    //      Replicas {
    //          hash_range: 405901753359583263..=6291554157536183168,    
    //          nodes: [Node { ip: 127.0.0.1 }, Node { ip: 127.0.0.3 }]
    //      },
    //      Replicas {
    //          hash_range: 6291554157536183169..=7034287380452369431,   
    //          nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.2 }]
    //      },
    //      Replicas {
    //          hash_range: 7034287380452369432..=8537067609243575564,   
    //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
    //      },
    //      Replicas {
    //          hash_range: 8537067609243575565..=11006066246803680578,  
    //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
    //      },
    //      Replicas {
    //          hash_range: 11006066246803680579..=15043474181722320697,
     //         nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.1 }]
     //      }
    // ]
    println!("{:?}", ring.get_hash_ranges());

    let mut ring2 = ring.clone();
    
    let new_node = Node::new("127.0.0.4");
    ring2.add(new_node.clone());

    // return instructions (hashranges and nodes as given in struct Replicas)
    // to copy/move keys from ring1 to ring2
    // to replicate all keys to new_node that need to be stored there
    // prints
    //  [
    //      Replicas {
    //          hash_range: 11006066246803680579..=12253783648769497289,
    //          nodes: [Node { ip: 127.0.0.3 }, Node { ip: 127.0.0.1 }]
    //      },
    //      Replicas {
    //          hash_range: 7034287380452369432..=11006066246803680578,
    //          nodes: [Node { ip: 127.0.0.2 }, Node { ip: 127.0.0.3 }]
    //      }
    //  ]
    println!("{:?}", ring2.find_sources(&new_node, &ring, &nodes));
}
```