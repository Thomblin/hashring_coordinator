//! A minimal implementation to coordinate replication and read/write requests within a hashring using Consistent Hashring
//! The coordinator needs to be updated when nodes join the ring, leave or left the ring
//! The coordinator can instruct the replication as well, if the complete ring needs to be replaced (due to a deployment for example)
//!
//! Nodes are described by a Hash and a State
//! The Hash is used to place the Node on the Hashring
//! The State is used to instruct all Nodes on the hashring which replications are needed to reach a stable state
//!     State New: A node that recently spawned and needs to sync state. It will not receive read requests. It could receive write requests though
//!     State Operational: A node that stores all required keys and is able to process read and write requests
//!     State Terminating: Optional state that can be used to sync state from this node other nodes, before it leaves the cluster
//!
//! Prerequisites:
//! A cluster of nodes will receive write read and write requests only after all nodes reached the state operational.
//!

use std::{fmt::Display, hash::Hash};

use hashring::HashRing;

#[derive(PartialEq)]
enum State {
    New, //  A node that recently spawned and needs to sync state. It will not receive read requests. It could receive write requests though
    Operational, // A node that stores all required keys and is able to process read and write requests
    Terminating, // Optional state that can be used to sync state from this node to other nodes, before it leaves the cluster
}

pub struct Node<T>
where
    T: Hash,
{
    node: T,
    state: State,
}

impl<T> Node<T>
where
    T: Hash,
{
    fn new(node: T, state: State) -> Node<T> {
        Node { node, state }
    }
}

impl<T> Hash for Node<T>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

pub struct Config {
    pub virtual_nodes: i32, // number of virtual nodes to create per real node
}

#[derive(Debug)]
pub enum Error {
    ClusterNotOperational,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

/// Coordinator takes care of all nodes of one cluster that belong to the same deployment
/// it can be combined with the Coordinator of another deployment to calculate actions needed to sync the state between both deployments (clusters)
pub struct Coordinator<T>
where
    T: Hash,
{
    config: Config,
    ring: HashRing<Node<T>>,
    operational: bool,
}

impl<T> Coordinator<T>
where
    T: Hash,
{
    /// initiate a new Coordinator
    pub fn new(config: Config) -> Coordinator<T> {
        Self {
            config,
            ring: HashRing::default(),
            operational: false,
        }
    }

    /// update the current status of all known nodes
    /// it needs to contain all alive nodes
    pub fn update(&mut self, nodes: Vec<Node<T>>) {
        let mut ring = HashRing::default();
        let operational = nodes.iter().all(|n| n.state == State::Operational);
        ring.batch_add(nodes);

        // TODO: publish actions based on the differences of the new and the current ring

        // Update the coordinator's ring map
        self.ring = ring;
        self.operational = operational;
    }

    /// return the target Nodes that should contain the requested key
    /// returns an error, if not all nodes of the cluster have been operational at least once at the same time yet
    pub fn get(&self, key: T) -> Result<Vec<Node<T>>, Error> {
        if !self.operational {
            return Err(Error::ClusterNotOperational);
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Config, Coordinator, Node, State};

    #[test]
    fn new_deployment() {
        let config = Config { virtual_nodes: 3 };
        let mut coordinator = Coordinator::new(config);

        let node1 = Node::new("1", State::New);
        let node2 = Node::new("2", State::New);
        let node3 = Node::new("3", State::New);

        coordinator.update(vec![node1, node2, node3]);

        let target = coordinator.get("1234");

        match target {
            Err(crate::Error::ClusterNotOperational) => (),
            _ => panic!(
                "coordinator should throw Error::ClusterNotOperational if cluster has not been operational yet"
            ),
        }
    }
}
