//! brief example how HashRing can be used to coordinate changes within a cluster if
//! - nodes are added to the cluster
//! - nodes are removed from the cluster
//! - a new cluster is deployed and all keys need to be replicated to the new deployment

extern crate hashring_coordinator;

use hashring_coordinator::{HashRing, Replicas};
use rand::{Rng, distr::Alphanumeric};
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::ops::RangeInclusive;
use std::str::FromStr;

fn main() {
    // amount of replicas. Each key is stored on (1 + replicas) nodes
    let replicas = 2;

    // amount of virtual nodes in the hashring.
    // Higher number achieves a more evenly distribution of all keys on the hashring
    // Higher number increases the computation overhead for replication
    let num_vnodes = 20;

    // create our initial cluster for testing
    let mut coordinator = Coordinator::new(
        replicas,
        num_vnodes,
        vec![
            VNode::new("127.0.0.1"),
            VNode::new("127.0.0.2"),
            VNode::new("127.0.0.3"),
            VNode::new("127.0.0.4"),
            VNode::new("127.0.0.5"),
        ],
    );

    // store all known keys to test later if we can retrieve all of them
    let mut known_keys = vec![];

    // store some random values in our cluster
    for _ in 0..10000 {
        let value = random_string();
        let key = format!("key_{value}"); // reuse the value to simplify our tests later
        known_keys.push(key.clone());
        coordinator.post(key, value);
    }

    // assert that all keys can be retrieved
    for key in &known_keys {
        match coordinator.test_get(key) {
            Ok(_) => (),
            Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
        }
    }

    // just a test to ensure that our assert pattern works
    match coordinator.test_get(&"foo".to_string()) {
        Ok(_) => println!("error: foo should not be found "),
        Err(mismatches) if mismatches == 1 + replicas => (),
        Err(mismatches) => println!(
            "error: foo should not be found on exactly {} nodes got {mismatches} instead",
            1 + replicas
        ),
    }

    // we need to keep a copy of our current hashring
    // to be able to compare the changes and perform a synchronization (replication)
    let hashring_previous = coordinator.hashring();

    // add a new node to the cluster, it should be empty until we simulate the replication
    coordinator.add_node(VNode::new("127.0.0.6"));

    // showcase the distribution of all values across the cluster
    println!("\n# distribution of keys across cluster, after new node joined");
    coordinator.print_utilization();

    // uncomment these lines, to check that several keys cannot be found
    /*  for key in &known_keys {
           match coordinator.test_get(key) {
               Ok(_) => (),
               Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
           }
       }
    */

    // calculate the differences of previous state and new state
    // and replicate all keys to new nodes if needed
    // here it will copy entries to the new node @ 127.0.0.6
    //
    // currently hashring_coordinator will not act on add_node or remove_node operations
    // as in real life scenarios it might be simpler to detect the current state of the cluster
    // and compare it with a previous state
    let available_nodes = hashring_previous.nodes();
    coordinator.rebalance(&hashring_previous, &available_nodes);

    // showcase the distribution of all values across the cluster after synchronizing to the new node
    println!("\n# distribution of keys across cluster, after new node was synchronized");
    coordinator.print_utilization();

    // assert that all keys can be retrieved
    for key in &known_keys {
        match coordinator.test_get(key) {
            Ok(_) => (),
            Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
        }
    }

    // we need to keep a copy of our current hashring
    // to be able to compare the changes and perform a synchronization (replication)
    let hashring_previous = coordinator.hashring();

    // drop a new node from the cluster
    coordinator.drop_node(VNode::new("127.0.0.3"));

    // uncomment these lines, to check that several keys cannot be found
    // dropping a node requires a redistribution of keys
    /*  for key in &known_keys {
           match coordinator.test_get(key) {
               Ok(_) => (),
               Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
           }
       }
    */

    // rebalance the hashring
    let available_nodes = coordinator.hashring.nodes();
    coordinator.rebalance(&hashring_previous, &available_nodes);

    // assert that all keys can be retrieved
    for key in &known_keys {
        match coordinator.test_get(key) {
            Ok(_) => (),
            Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
        }
    }

    // showcase the distribution of all values across the cluster after rebalancing
    println!("\n# distribution of keys across cluster, after one node left");
    coordinator.print_utilization();

    // simulate a deployment, spawn a new, empty cluster
    // different IPs will generate completely different hashranges
    // so that keys need to be synchronized crisscross
    // as well, for the showcase, our new cluster contains 4 instead of 5 nodes
    let mut coordinator_replacement = Coordinator::new(
        replicas,
        num_vnodes,
        vec![
            VNode::new("127.0.0.11"),
            VNode::new("127.0.0.12"),
            VNode::new("127.0.0.13"),
            VNode::new("127.0.0.14"),
        ],
    );

    // synchronize all entries from the first cluster to the new one
    coordinator_replacement.synchronize(&coordinator);

    // showcase the distribution of all values across the new cluster after synchronization
    println!("\n# distribution of keys across the new cluster");
    coordinator_replacement.print_utilization();

    // assert that all keys can be retrieved
    for key in &known_keys {
        match coordinator_replacement.test_get(key) {
            Ok(_) => (),
            Err(mismatches) => println!("error: {key} not found on {mismatches} nodes"),
        }
    }
}

/// struct VNode represent a Node of our cluster to perform calculations in HashRing
#[derive(Clone, Debug, Hash, PartialEq)]
struct VNode {
    ip: IpAddr,
}

impl VNode {
    fn new(ip: &str) -> Self {
        VNode {
            ip: IpAddr::from_str(ip).unwrap(),
        }
    }
}

/// struct Node represents a real server node of our cluster. It will store and deliver key/value pairs
struct Node {
    store: HashMap<String, String>,
}

impl Node {
    fn new() -> Self {
        Node {
            store: HashMap::new(),
        }
    }

    /// simulate a http POST call to store a given key/value pair
    fn post(&mut self, key: String, value: String) {
        self.store.insert(key, value);
    }

    /// simulate a http GET call to retrieve the value for given key
    fn get(&self, key: &String) -> Option<&String> {
        self.store.get(key)
    }

    /// returns all (key, value) pairs where hash(key) is included in hash_range
    fn fetch_range(
        &self,
        hash_range: RangeInclusive<u64>,
        hashring: &HashRing<VNode>,
    ) -> Vec<(String, String)> {
        self.store
            .iter()
            .filter(|(key, _)| hash_range.contains(&hashring.get_hash(key)))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// returns the amount of stored values
    fn size(&self) -> usize {
        self.store.len()
    }
}

/// struct Coordinator keeps track of all nodes in our cluster
/// it will redirect post and get calls to the nodes responsible for a given key
/// it will synchronize entries across nodes when neeeded
///
/// * `hashring` - contains representatives of all nodes in our cluster
/// * `nodes` - represent the actual nodes of our cluster, these will store all values
struct Coordinator {
    hashring: HashRing<VNode>,
    nodes: HashMap<IpAddr, Node>,
}

impl Coordinator {
    /// create a new coordinator for the given cluster
    fn new(replicas: usize, num_vnodes: usize, vnodes: Vec<VNode>) -> Self {
        let mut hashring = HashRing::new(replicas, num_vnodes);

        let mut nodes = HashMap::new();

        for vnode in &vnodes {
            nodes.insert(vnode.ip, Node::new());
        }

        hashring.batch_add(vnodes);

        Coordinator { hashring, nodes }
    }

    /// add a new node to our cluster
    fn add_node(&mut self, vnode: VNode) {
        self.nodes.insert(vnode.ip, Node::new());
        self.hashring.add(vnode);
    }

    /// remove a node from our cluster
    fn drop_node(&mut self, vnode: VNode) {
        self.nodes.retain(|ip, _| *ip != vnode.ip);
        self.hashring.remove(&vnode);
    }

    /// simulate a http POST call to store a given key/value pair
    fn post(&mut self, key: String, value: String) {
        let vnodes = self.hashring.get(&key);

        for vnode in vnodes {
            // println!("store {key}:{value} to {vnode:?}");

            self.nodes
                .entry(vnode.ip)
                .and_modify(|n| n.post(key.clone(), value.clone()));
        }
    }

    /// simulate a http GET call to retrieve the value for given key
    /// function tests to retrieve the value from all nodes that should contain the key a
    /// returns OK if all keys were found, or Err(usize) with amount of nodes that did not return the expected value
    fn test_get(&self, key: &String) -> Result<(), usize> {
        let vnodes = self.hashring.get(&key);

        let mut value = None;
        let mut mismatch = 0;

        for vnode in &vnodes {
            let val = self.get_node(vnode).get(key);

            match val {
                Some(val) if value.is_none() => value = Some(val),
                Some(val) if value == Some(val) => (),
                Some(_) => mismatch += 1,
                None => mismatch += 1,
            }
        }

        match mismatch {
            0 => Ok(()),
            _ => Err(mismatch),
        }
    }

    // returns the "real" node for a given vnode of the hashring
    // panics if the node could not be found,
    // in our example we expect that vnodes and nodes are always in sync
    fn get_node(&self, vnode: &VNode) -> &Node {
        self.nodes
            .get(&vnode.ip)
            .unwrap_or_else(|| panic!("expected to find node {}", vnode.ip))
    }

    /// print for each node how many values are currently stored
    fn print_utilization(&self) {
        for (ip, node) in &self.nodes {
            println!("{ip} contains {} values", node.size())
        }
    }

    /// retrieve a copy of the current hashring
    fn hashring(&self) -> HashRing<VNode> {
        self.hashring.clone()
    }

    /// synchronize entries inside this cluster
    /// based on the changes / difference to the provided (previous) HashRing
    fn rebalance(&mut self, from: &HashRing<VNode>, available_nodes: &[VNode]) {
        for target_vnode in &self.hashring {
            let instructions = self
                .hashring
                .find_sources(target_vnode, from, available_nodes);

            for Replicas { hash_range, nodes } in instructions {
                // fetch all values from the first source node
                // in the real world you might iterate over all nodes
                // or use the remaining nodes as fallback, if a node is not responsive
                if let Some(source_vnode) = nodes.first() {
                    let values = self
                        .get_node(source_vnode)
                        .fetch_range(hash_range, &self.hashring);

                    if let Some(target_node) = self.nodes.get_mut(&target_vnode.ip) {
                        // copy all values to target_node
                        for (key, value) in values {
                            target_node.post(key.clone(), value.clone())
                        }
                    }
                }
            }
        }
    }

    /// synchronize entries from another cluster into this cluster
    fn synchronize(&mut self, from: &Coordinator) {
        for target_vnode in &self.hashring {
            let instructions =
                self.hashring
                    .find_sources(target_vnode, &from.hashring, &from.hashring.nodes());

            for Replicas { hash_range, nodes } in instructions {
                // fetch all values from the first source node
                // in the real world you might iterate over all nodes
                // or use the remaining nodes as fallback, if a node is not responsive
                if let Some(source_vnode) = nodes.first() {
                    let values = from
                        .get_node(source_vnode)
                        .fetch_range(hash_range, &self.hashring);

                    if let Some(target_node) = self.nodes.get_mut(&target_vnode.ip) {
                        // copy all values to target_node
                        for (key, value) in values {
                            target_node.post(key.clone(), value.clone())
                        }
                    }
                }
            }
        }
    }
}

/// generate a random String to test that our values are stored, retrieved and replicated correctly
fn random_string() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}
