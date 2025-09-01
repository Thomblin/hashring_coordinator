use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::RangeInclusive;

use super::HashRing;

#[derive(Clone, Debug, PartialEq)]
struct Replicas<T> {
    hash_range: RangeInclusive<u64>,
    nodes: Vec<T>,
}

impl<T> HashRing<T>
where
    T: Hash + Clone + Debug + PartialEq,
{
    fn get_hash_ranges(&self) -> Vec<Replicas<T>> {
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

    /// for given node: Node calculate all available source Nodes that can provide keys which need to be stored on the given node
    /// available nodes provides all nodes that can be used to replicate keys from
    /// this covers different scenarios:
    /// 1. from and to cover the same deployment of one cluster, containing nodes that left or joined the ring
    /// 2. from and to cover the same deployment of one cluster, containing some nodes that are terminating currently (available to sync from, but should not receive new keys)
    /// 3. from refering to the original deployment and to refering to the replacement deployment, therefor available_nodes refering to the original deployment
    fn find_sources(
        &self,
        node: &T,
        available_nodes: &[T],
        from: &Vec<Replicas<T>>,
        to: &Vec<Replicas<T>>,
    ) -> Vec<Replicas<T>> {
        let mut sources = vec![];

        for needed in to {
            if !needed.nodes.contains(node) {
                continue;
            }

            for supply in from {
                if let Some(range) = intersect(&needed.hash_range, &supply.hash_range) {
                    let mut nodes = supply.nodes.clone();
                    nodes.retain(|f| available_nodes.contains(f));

                    sources.push(Replicas {
                        hash_range: range,
                        nodes,
                    });
                }
            }
        }

        self.merge_replicas(sources)
    }

    fn merge_replicas(&self, mut replicas: Vec<Replicas<T>>) -> Vec<Replicas<T>> {
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

    fn hash_nodes(&self, nodes: &Vec<T>) -> u64 {
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
    use std::hash::{Hash, Hasher};
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use crate::hashring::HashRing;
    use crate::hashring::coordinator::Replicas;

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
    fn hash_ranges_should_be_calculated_correctly() {
        let node1 = Node::new("127.0.0.1"); // @1093046220658055553
        let node4 = Node::new("127.0.0.4"); // @7061776985767999842
        let node2 = Node::new("127.0.0.2"); // @7508079630756128442
        let node3 = Node::new("127.0.0.3"); // @12322253174093194230

        let nodes_current = vec![node1, node2, node3];
        let mut ring_current = HashRing::new(0, 1);
        ring_current.batch_add(nodes_current);

        let hash1 = ring_current.get_hash((&node1, 0_usize));
        assert_eq!(1093046220658055553, hash1);
        let hash2 = ring_current.get_hash((&node2, 0_usize));
        assert_eq!(7508079630756128442, hash2);
        let hash3 = ring_current.get_hash((&node3, 0_usize));
        assert_eq!(12322253174093194230, hash3);
        let hash4 = ring_current.get_hash((&node4, 0_usize));
        assert_eq!(7061776985767999842, hash4);

        let nodes_new = vec![node1, node2, node3, node4];
        let mut ring_new = HashRing::new(0, 1);
        ring_new.batch_add(nodes_new.clone());

        let replica_setup_original = ring_current.get_hash_ranges();

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

        let sources = ring_new.find_sources(
            &node1,
            &nodes_new,
            &replica_setup_original,
            &replica_setup_new,
        );
        let expected = vec![
            Replicas {
                hash_range: 0..=hash1,
                nodes: vec![node1],
            },
            Replicas {
                hash_range: (hash3 + 1)..=u64::MAX,
                nodes: vec![node1],
            },
        ];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(
            &node4,
            &nodes_new,
            &replica_setup_original,
            &replica_setup_new,
        );
        let expected = vec![Replicas {
            hash_range: (hash1 + 1)..=hash4,
            nodes: vec![node2],
        }];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(
            &node2,
            &nodes_new,
            &replica_setup_original,
            &replica_setup_new,
        );
        let expected = vec![Replicas {
            hash_range: (hash4 + 1)..=hash2,
            nodes: vec![node2],
        }];
        assert_eq!(expected, sources);

        let sources = ring_new.find_sources(
            &node3,
            &nodes_new,
            &replica_setup_original,
            &replica_setup_new,
        );
        let expected = vec![Replicas {
            hash_range: (hash2 + 1)..=hash3,
            nodes: vec![node3],
        }];
        assert_eq!(expected, sources);
    }
}
