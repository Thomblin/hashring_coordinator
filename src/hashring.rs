extern crate siphasher;

use siphasher::sip::SipHasher;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::BuildHasher;

pub mod coordinator;
mod crud;
mod iterator;

#[derive(Clone, PartialEq, Debug)]
pub struct DefaultHashBuilder;

impl BuildHasher for DefaultHashBuilder {
    type Hasher = SipHasher;

    fn build_hasher(&self) -> Self::Hasher {
        SipHasher::new()
    }
}

// Node is an internal struct used to encapsulate the nodes that will be added and
// removed from `HashRing`
#[derive(Clone, Debug)]
struct Node<T> {
    key: u64,
    node: T,
    virtual_id: usize,
}

impl<T> Node<T> {
    fn new(key: u64, node: T, virtual_id: usize) -> Node<T> {
        Node {
            key,
            node,
            virtual_id,
        }
    }
}

// Implement `PartialEq`, `Eq`, `PartialOrd` and `Ord` so we can sort `Node`s
impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for Node<T> {}

impl<T> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Node<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Node<T> {
    fn cmp(&self, other: &Node<T>) -> Ordering {
        self.key.cmp(&other.key)
    }
}

/// HashRing represents a set of nodes (cluster) that shall use consistent hashing
/// HashRing provides methods to add and remove nodes to the cluster
/// HashRing can calculate for each node which hashranges they are responsible for
/// HashRing can calculate replication instructions if a cluster changes or if the cluster if replaced completely to find target nodes and source nodes with affected hashranges
#[derive(Clone, PartialEq, Debug)]
pub struct HashRing<T, S = DefaultHashBuilder> {
    hash_builder: S,
    ring: Vec<Node<T>>,
    replicas: usize,
    vnodes: usize,
}

impl<T> Default for HashRing<T> {
    fn default() -> Self {
        HashRing {
            hash_builder: DefaultHashBuilder,
            ring: Vec::new(),
            replicas: 2,
            vnodes: 200,
        }
    }
}

/// Hash Ring
///
/// A hash ring that provides consistent hashing for nodes that are added to it.
impl<T> HashRing<T> {
    /// Create a new `HashRing`.
    ///
    /// # Arguments
    ///
    /// * `replicas` - number of nodes to store copies of each key (set replicas to 0, to store each key only once)
    /// * `vnodes` - number of virtual nodes per real node in the cluster (higher number means more even distribution of keys across all nodes, but higher processing effort)
    pub fn new(replicas: usize, vnodes: usize) -> HashRing<T> {
        HashRing {
            hash_builder: DefaultHashBuilder,
            ring: Vec::new(),
            replicas,
            vnodes: vnodes.max(1),
        }
    }
}

impl<T, S> HashRing<T, S> {
    /// Get the number of real nodes in the hash ring.
    pub fn len(&self) -> usize {
        self.ring.len() / self.vnodes
    }

    /// Get the number of virtual nodes in the hash ring.
    pub fn vlen(&self) -> usize {
        self.ring.len()
    }

    /// Returns true if the ring has no elements.
    pub fn is_empty(&self) -> bool {
        self.ring.len() == 0
    }
    /// Creates an empty `HashRing` which will use the given hash builder.
    ///
    /// # Arguments
    ///
    /// * `replicas` - number of nodes to store copies of each key (set replicas to 0, to store each key only once)
    /// * `vnodes` - number of virtual nodes per real node in the cluster (higher number means more even distribution of keys across all nodes, but higher processing effort)
    /// * `hash_builder` - implementation of BuildHasher to provider a Hasher for the HashRing
    ///
    /// # Examples
    ///
    /// ```
    /// use hashring_coordinator::HashRing;
    /// use siphasher::sip::SipHasher;
    /// use std::hash::{BuildHasher, Hash, Hasher};
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// pub struct DefaultHashBuilder;
    ///
    /// impl BuildHasher for DefaultHashBuilder {
    ///     type Hasher = SipHasher;
    ///
    ///     fn build_hasher(&self) -> Self::Hasher {
    ///         SipHasher::new()
    ///     }
    /// }
    ///
    /// struct Node {}
    /// impl Hash for Node {
    ///     fn hash<H: Hasher>(&self, s: &mut H) {
    ///         todo!()
    ///     }
    /// }
    ///
    /// let hash_builder = DefaultHashBuilder {};
    /// let mut ring: HashRing<Node, DefaultHashBuilder> = HashRing::with_hasher(2, 100, hash_builder);
    /// ```
    pub fn with_hasher(replicas: usize, vnodes: usize, hash_builder: S) -> HashRing<T, S> {
        HashRing {
            hash_builder,
            ring: Vec::new(),
            replicas,
            vnodes,
        }
    }
}
