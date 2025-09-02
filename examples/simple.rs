//! basic example to showcase the main functions of HashRing

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
    let mut ring: HashRing<Node> = HashRing::new(1, 2);
    let nodes = vec![
        Node::new("127.0.0.1"),
        Node::new("127.0.0.2"),
        Node::new("127.0.0.3"),
    ];

    ring.batch_add(nodes.clone());

    // return list of nodes that store the key 'foo'
    println!("hash for key foo: {:?}", ring.get(&"foo"));

    // return Vec<Replicas> containing hash ranges for each node of the cluster, defining which keys to store or find where
    println!(
        "hash_ranges for cluster [127.0.0.1, 127.0.0.2. 127.0.0.3]:\n{:?}",
        ring.get_hash_ranges()
    );

    let mut ring2 = ring.clone();
    let new_node = Node::new("127.0.0.4");
    ring2.add(new_node.clone());

    // return instructions (hash_ranges and nodes as given in struct Replicas) to copy/move keys from ring1 to ring2 to replicate all keys to new_node that need to be stored there
    println!(
        "replicate keys in hashrange from given nodes to new node:\n{:?}",
        ring2.find_sources(&new_node, &ring, &nodes)
    );
}
