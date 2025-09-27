use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::RangeInclusive;

#[cfg(feature = "derive")]
use serde::{Deserialize, Serialize};

use super::HashRing;

/// Replicas contains a hashrange and all nodes that store keys within the given range
/// The first node in `nodes` is the primary node, the following nodes are replication nodes
///
/// * `hash_range` - range of hashes that are stored on the given nodes. Careful: Multiple hashranges might apply to each node. You need to consider all given Replicas structs
/// * `nodes` - all nodes that store keys with a hash in hash_range
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
pub struct Replicas<T> {
    pub hash_range: RangeInclusive<u64>,
    pub nodes: Vec<T>,
}

impl<T> HashRing<T>
where
    T: Hash + Clone + Debug + PartialEq,
{
    pub fn get_hash_ranges(&self) -> Vec<Replicas<T>> {
        if self.len() == 1 {
            return vec![Replicas {
                hash_range: 0..=u64::MAX,
                nodes: vec![self.ring.first().unwrap().node.clone()],
            }];
        }

        let mut replication_setup = vec![];

        let mut left = match self.ring.last() {
            Some(left) => left.clone(),
            None => {
                return replication_setup;
            }
        };

        for right in self.ring.iter() {
            if left.key > right.key {
                replication_setup.push(Replicas {
                    hash_range: left.key + 1..=u64::MAX,
                    nodes: self.get(&(right.node.clone(), right.virtual_id)),
                });
                replication_setup.push(Replicas {
                    hash_range: 0..=right.key,
                    nodes: self.get(&(right.node.clone(), right.virtual_id)),
                });
            } else {
                replication_setup.push(Replicas {
                    hash_range: left.key + 1..=right.key,
                    nodes: self.get(&(right.node.clone(), right.virtual_id)),
                });
            }

            left = right.clone();
        }

        replication_setup
    }

    /// for given node: Node calculate all available source Nodes that can provide keys which need to be stored on the given node (after a change of the given cluster)
    /// for general replication instructions within the cluster, use fn get_hash_ranges()
    /// available nodes provides all nodes that can be used to replicate keys from
    /// this covers different scenarios:
    ///
    /// 1. `self` (this HashRing) and `source` cover the same deployment of one cluster, containing nodes that left or joined the ring
    /// 2. `self` (this HashRing) and `source` cover the same deployment of one cluster, containing some nodes that are terminating currently (available to sync from, but should not receive new keys)
    /// 3. `source` refering to the original deployment and `self` (this HashRing) refering to the replacement deployment, therefor available_nodes refering to the original deployment
    ///
    ///
    /// # Arguments
    ///
    /// * `target` - return replication sources for this node. target needs to be member of the current HashRing
    /// * `source` - find replication nodes within this HashRing
    /// * `available_nodes` - define all nodes that can be used for replication in source HashRing
    ///
    /// # Examples
    ///
    /// ```    
    /// use std::hash::{Hash, Hasher};
    /// use std::net::Ipv4Addr;
    /// use std::str::FromStr;
    ///
    /// use hashring_coordinator::HashRing;
    ///
    /// #[derive(Debug, Copy, Clone, PartialEq)]
    /// struct Node {
    ///     addr: Ipv4Addr,
    /// }
    ///
    /// impl Node {
    ///     fn new(ip: &str) -> Self {
    ///         let addr = Ipv4Addr::from_str(ip).unwrap();
    ///         Node { addr }
    ///     }
    /// }
    ///
    /// impl Hash for Node {
    ///     fn hash<H: Hasher>(&self, s: &mut H) {
    ///         (self.addr).hash(s)
    ///     }
    /// }
    ///  
    ///
    /// let node1 = Node::new("127.0.0.1"); // hash @1093046220658055553
    /// let node2 = Node::new("127.0.0.2"); // hash @7508079630756128442
    /// let node3 = Node::new("127.0.0.3"); // hash @12322253174093194230
    ///
    /// let nodes_original = vec![node1, node2];
    /// let mut ring_original = HashRing::new(0, 1);
    ///
    /// ring_original.batch_add(nodes_original.clone());
    ///
    /// let mut ring_new = ring_original.clone();
    /// ring_new.add(node3.clone());
    ///
    /// // sources contains a list with hashranges and target nodes that can be synchronized to node3
    /// // sources = [Replicas { hash_range: 7508079630756128443..=12322253174093194230, nodes: [Node { addr: 127.0.0.1 }] }]
    /// // node3 was added, thus the hashrange between node2 and node3 which was located on node1 so far can be moved over to node3
    /// let sources = ring_new.find_sources(&node3, &ring_original, &nodes_original);
    ///
    ///
    /// ```
    pub fn find_sources(
        &self,
        target: &T,
        source: &HashRing<T>,
        available_nodes: &[T],
    ) -> Vec<Replicas<T>> {
        let mut sources = vec![];

        let from = source.get_hash_ranges();
        let to = self.get_hash_ranges();

        for needed in to {
            if !needed.nodes.contains(target) {
                continue;
            }

            for supply in from.iter() {
                if let Some(range) = intersect(&needed.hash_range, &supply.hash_range) {
                    let mut nodes = supply.nodes.clone();
                    nodes.retain(|f| available_nodes.contains(f));

                    if nodes.contains(target) {
                        continue;
                    }

                    sources.push(Replicas {
                        hash_range: range,
                        nodes,
                    });
                }
            }
        }

        self.merge_replicas(sources)
    }

    pub fn merge_replicas(&self, mut replicas: Vec<Replicas<T>>) -> Vec<Replicas<T>> {
        replicas.sort_by(|a, b| a.hash_range.start().cmp(b.hash_range.start()));

        let mut replica_sets: HashMap<u64, Vec<Replicas<T>>> = HashMap::new();

        for replica in replicas.into_iter() {
            let hash = self.hash_nodes(&replica.nodes);
            let entry = replica_sets.entry(hash).or_default();
            entry.push(replica);
        }

        let mut combined = vec![];

        for (_, mut set) in replica_sets.into_iter() {
            let set_tail = set.split_off(1);
            let mut current = match set.pop() {
                Some(current) => current,
                None => continue,
            };

            for r in set_tail {
                if *current.hash_range.end() < u64::MAX
                    && *r.hash_range.start() == current.hash_range.end() + 1
                {
                    current.hash_range = *current.hash_range.start()..=*r.hash_range.end();
                } else {
                    combined.push(current);
                    current = r;
                }
            }
            combined.push(current);
        }

        combined
    }

    pub fn hash_nodes(&self, nodes: &Vec<T>) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        for node in nodes {
            node.hash(&mut hasher);
        }
        hasher.finish()
    }
}

fn intersect<T: Ord + Copy>(
    a: &RangeInclusive<T>,
    b: &RangeInclusive<T>,
) -> Option<RangeInclusive<T>> {
    let start = *a.start().max(b.start());
    let end = *a.end().min(b.end());

    if start <= end {
        Some(start..=end)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::hashring::HashRing;
    use crate::hashring::coordinator::Replicas;
    use pretty_assertions::assert_eq;
    use std::hash::{Hash, Hasher};
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    #[derive(Debug, Copy, Clone, PartialEq)]
    struct Node {
        addr: Ipv4Addr,
    }

    impl Node {
        fn new(ip: &str) -> Self {
            let addr = Ipv4Addr::from_str(ip).unwrap();
            Node { addr }
        }
    }

    impl Hash for Node {
        fn hash<H: Hasher>(&self, s: &mut H) {
            (self.addr).hash(s)
        }
    }

    #[test]
    fn hash_ranges_find_sources_minimal() {
        let node1 = Node::new("127.0.0.1"); // @1093046220658055553
        let node2 = Node::new("127.0.0.2"); // @7508079630756128442

        let mut ring_original = HashRing::new(0, 1);
        ring_original.add(node2);

        let mut ring_new = HashRing::new(0, 1);
        ring_new.add(node1);

        let ring_original_hashrange = ring_original.get_hash_ranges();

        let expected_original = vec![Replicas {
            hash_range: 0..=u64::MAX,
            nodes: vec![node2],
        }];

        assert_eq!(expected_original, ring_original_hashrange);

        let ring_new_hashrange = ring_new.get_hash_ranges();

        let expected_new = vec![Replicas {
            hash_range: 0..=u64::MAX,
            nodes: vec![node1],
        }];

        assert_eq!(expected_new, ring_new_hashrange);

        let sources = ring_new.find_sources(&node1, &ring_original, &[node2]);
        let expected: Vec<Replicas<Node>> = vec![Replicas {
            hash_range: 0..=u64::MAX,
            nodes: vec![node2],
        }];
        assert_eq!(expected, sources);
    }

    #[test]
    fn hash_ranges_should_be_calculated_correctly() {
        let node1 = Node::new("127.0.0.1"); // @1093046220658055553
        let node4 = Node::new("127.0.0.4"); // @7061776985767999842
        let node2 = Node::new("127.0.0.2"); // @7508079630756128442
        let node3 = Node::new("127.0.0.3"); // @12322253174093194230

        let nodes_original = vec![node1, node2, node3];
        let mut ring_original = HashRing::new(0, 1);
        ring_original.batch_add(nodes_original.clone());

        let hash1 = ring_original.get_hash(&(&node1, 0_usize));
        assert_eq!(1093046220658055553, hash1);
        let hash2 = ring_original.get_hash(&(&node2, 0_usize));
        assert_eq!(7508079630756128442, hash2);
        let hash3 = ring_original.get_hash(&(&node3, 0_usize));
        assert_eq!(12322253174093194230, hash3);
        let hash4 = ring_original.get_hash(&(&node4, 0_usize));
        assert_eq!(7061776985767999842, hash4);

        let nodes_new = vec![node1, node2, node3, node4];
        let mut ring_new = HashRing::new(0, 1);
        ring_new.batch_add(nodes_new.clone());

        let replica_setup_original = ring_original.get_hash_ranges();

        let expected_original = vec![
            Replicas {
                hash_range: (hash3 + 1)..=u64::MAX,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: 0..=hash1,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: (hash1 + 1)..=hash2,
                nodes: vec![node2],
            },
            Replicas {
                hash_range: (hash2 + 1)..=hash3,
                nodes: vec![node3],
            },
        ];

        assert_eq!(expected_original, replica_setup_original);

        let replica_setup_new = ring_new.get_hash_ranges();

        let expected_new = vec![
            Replicas {
                hash_range: (hash3 + 1)..=u64::MAX,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: 0..=hash1,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: (hash1 + 1)..=hash4,
                nodes: vec![node4],
            },
            Replicas {
                hash_range: (hash4 + 1)..=hash2,
                nodes: vec![node2],
            },
            Replicas {
                hash_range: (hash2 + 1)..=hash3,
                nodes: vec![node3],
            },
        ];

        assert_eq!(expected_new, replica_setup_new);

        let sources = ring_new.find_sources(&node1, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node4, &ring_original, &nodes_original);
        let expected = vec![Replicas {
            hash_range: (hash1 + 1)..=hash4,
            nodes: vec![node2],
        }];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node2, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node3, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);
    }

    #[test]
    fn hash_ranges_should_be_calculated_correctly_with_replicas() {
        let node1 = Node::new("127.0.0.1"); // id = 0  @1093046220658055553, id = 1 @10619849754955980960
        let node2 = Node::new("127.0.0.2"); // id = 0  @7508079630756128442, id = 1  @7110299084231520957
        let node3 = Node::new("127.0.0.3"); // id = 0 @12322253174093194230, id = 1    @24307670534837389
        let node4 = Node::new("127.0.0.4"); // id = 0 @7061776985767999842,  id = 1  @1807640587661881848

        let nodes_original = vec![node1, node2, node3];
        let mut ring_original = HashRing::new(1, 2);
        ring_original.batch_add(nodes_original.clone());

        let hash3_1 = ring_original.get_hash(&(&node3, 1_usize));
        assert_eq!(24307670534837389, hash3_1);
        let hash1_0 = ring_original.get_hash(&(&node1, 0_usize));
        assert_eq!(1093046220658055553, hash1_0);
        let hash2_1 = ring_original.get_hash(&(&node2, 1_usize));
        assert_eq!(7110299084231520957, hash2_1);
        let hash2_0 = ring_original.get_hash(&(&node2, 0_usize));
        assert_eq!(7508079630756128442, hash2_0);
        let hash1_1 = ring_original.get_hash(&(&node1, 1_usize));
        assert_eq!(10619849754955980960, hash1_1);
        let hash3_0 = ring_original.get_hash(&(&node3, 0_usize));
        assert_eq!(12322253174093194230, hash3_0);

        let hash4_1 = ring_original.get_hash(&(&node4, 1_usize));
        assert_eq!(1807640587661881848, hash4_1);
        let hash4_0 = ring_original.get_hash(&(&node4, 0_usize));
        assert_eq!(7061776985767999842, hash4_0);

        let nodes_new = vec![node1, node2, node3, node4];
        let mut ring_new = HashRing::new(0, 1);
        ring_new.batch_add(nodes_new.clone());

        let replica_setup_original = ring_original.get_hash_ranges();

        let expected_original = vec![
            Replicas {
                hash_range: (hash3_0 + 1)..=u64::MAX,
                nodes: vec![node3, node1],
            },
            Replicas {
                hash_range: 0..=hash3_1,
                nodes: vec![node3, node1],
            },
            Replicas {
                hash_range: (hash3_1 + 1)..=hash1_0,
                nodes: vec![node1, node2],
            },
            Replicas {
                hash_range: (hash1_0 + 1)..=hash2_1,
                nodes: vec![node2, node1],
            },
            Replicas {
                hash_range: (hash2_1 + 1)..=hash2_0,
                nodes: vec![node2, node1],
            },
            Replicas {
                hash_range: (hash2_0 + 1)..=hash1_1,
                nodes: vec![node1, node3],
            },
            Replicas {
                hash_range: (hash1_1 + 1)..=hash3_0,
                nodes: vec![node3, node1],
            },
        ];

        assert_eq!(expected_original, replica_setup_original);

        let replica_setup_new = ring_new.get_hash_ranges();

        let expected_new = vec![
            Replicas {
                hash_range: (hash3_0 + 1)..=u64::MAX,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: 0..=hash1_0,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: (hash1_0 + 1)..=hash4_0,
                nodes: vec![node4],
            },
            Replicas {
                hash_range: (hash4_0 + 1)..=hash2_0,
                nodes: vec![node2],
            },
            Replicas {
                hash_range: (hash2_0 + 1)..=hash3_0,
                nodes: vec![node3],
            },
        ];

        assert_eq!(expected_new, replica_setup_new);

        let sources = ring_new.find_sources(&node1, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node4, &ring_original, &nodes_original);
        let expected = vec![Replicas {
            hash_range: (hash1_0 + 1)..=hash4_0,
            nodes: vec![node2, node1],
        }];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node2, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(&node3, &ring_original, &nodes_original);
        let expected: Vec<Replicas<Node>> = vec![];
        assert_eq!(expected, sources);
    }
}
