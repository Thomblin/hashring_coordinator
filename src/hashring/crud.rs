use std::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
};

use super::{HashRing, Node};

impl<T, S> HashRing<T, S>
where
    T: Hash + Clone + Debug,
    S: BuildHasher,
{
    /// Add `node` to the hash ring.
    pub fn add(&mut self, node: T) {
        self.add_virtual_nodes(node);
        self.ring.sort();
    }

    /// adds a real node represented by X virtual nodes to the hash ring
    fn add_virtual_nodes(&mut self, node: T) {
        for id in 0..self.vnodes {
            let key = self.get_hash((&node, id));
            self.ring.push(Node::new(key, node.clone(), id)); // TODO: avoid duplicates
        }
    }

    pub fn batch_add(&mut self, nodes: Vec<T>) {
        for node in nodes {
            self.add_virtual_nodes(node);
        }
        self.ring.sort()
    }

    /// Remove `node` from the hash ring.
    pub fn remove(&mut self, node: &T)
    where
        T: PartialEq,
    {
        self.ring.retain(|n| n.node != *node);
    }

    /// returns all real nodes responsible for `key`
    /// Returns an empty array if the ring is empty
    pub fn get<U: Hash>(&self, key: &U) -> Vec<T>
    where
        T: Clone + Debug + PartialEq,
    {
        if self.ring.is_empty() {
            return vec![];
        }

        let limit = (self.replicas + 1).min(self.len());

        let hash = self.get_hash(key);

        let n = match self.ring.binary_search_by(|node| node.key.cmp(&hash)) {
            Err(n) => n,
            Ok(n) => n,
        };

        let mut nodes = self.ring.clone();
        nodes.rotate_left(n);

        let mut replica_nodes = vec![];

        for node in nodes {
            if !replica_nodes.contains(&node.node) {
                replica_nodes.push(node.node.clone());

                if replica_nodes.len() == limit {
                    break;
                }
            }
        }

        replica_nodes
    }

    // An internal function for converting a reference to a hashable type into a `u64` which
    // can be used as a key in the hash ring.
    pub fn get_hash<U>(&self, input: U) -> u64
    where
        U: Hash,
    {
        self.hash_builder.hash_one(input)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::hash::BuildHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use super::HashRing;

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

    struct FixedBuildHasher {}

    impl BuildHasher for FixedBuildHasher {
        type Hasher = FixedHasher;

        fn build_hasher(&self) -> Self::Hasher {
            FixedHasher { hash: 0 }
        }
    }

    struct FixedHasher {
        hash: u64,
    }

    impl Hasher for FixedHasher {
        fn finish(&self) -> u64 {
            self.hash
        }

        fn write(&mut self, bytes: &[u8]) {
            let sum = bytes.iter().map(|b| *b as u64).sum::<u64>();

            self.hash += match self.hash {
                0 => sum * 7,
                _ => sum * 17,
            };
        }
    }

    #[test]
    fn test_that_hasher_returns_unique_hashes_for_small_ranges() {
        let hasher_builder = FixedBuildHasher {};
        let mut hashes = HashMap::new();

        for last_byte in 0..10 {
            for id in 0..10 {
                let ip = format!("127.0.0.{last_byte}");
                let node = Node::new(ip.as_str());
                let hash = hasher_builder.hash_one((&node, id));
                let key = format!("{ip} #{id}");

                match hashes.entry(hash) {
                    std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                        panic!("hash {hash} used by {} and {key}", occupied_entry.get())
                    }
                    std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(key)
                    }
                };
            }
        }
    }

    #[test]
    fn add_and_remove_nodes() {
        let hash_builder = FixedBuildHasher {};

        let mut ring: HashRing<Node, FixedBuildHasher> = HashRing::with_hasher(0, 3, hash_builder);

        assert_eq!(ring.len(), 0);
        assert!(ring.is_empty());

        let node1 = Node::new("127.0.0.1"); // hashes 896         913          930
        let node2 = Node::new("127.0.0.2"); // hashes     903         920          937
        let node3 = Node::new("127.0.0.3"); // hashes         910          927

        ring.add(node1);
        ring.add(node2);
        ring.add(node3);
        assert_eq!(ring.len(), 3);
        assert!(!ring.is_empty());

        ring.remove(&node2);
        assert_eq!(ring.len(), 2);

        let node4 = Node::new("127.0.0.4"); // hashes 917         934         951
        let node5 = Node::new("127.0.0.5"); // hashes     924         941         958
        let node6 = Node::new("127.0.0.6"); // hashes         931         948         965

        ring.batch_add(vec![node4, node5, node6]);

        assert_eq!(ring.len(), 5);
    }

    #[test]
    fn get_nodes() {
        let hash_builder = FixedBuildHasher {};

        let mut ring: HashRing<Node, FixedBuildHasher> = HashRing::with_hasher(0, 3, hash_builder);

        assert_eq!(ring.get(&"foo"), vec![]);

        let node1 = Node::new("127.0.0.1"); // hashes 896   913       930
        let node2 = Node::new("127.0.0.2"); // hashes   903     920         937
        let node3 = Node::new("127.0.0.3"); // hashes     910       927         944
        let node4 = Node::new("127.0.0.4"); // hashes         917         934       951
        let node5 = Node::new("127.0.0.5"); // hashes             924         941     958
        let node6 = Node::new("127.0.0.6"); // hashes                   931       948   965

        ring.add(node1);
        ring.add(node2);
        ring.add(node3);
        ring.add(node4);
        ring.add(node5);
        ring.add(node6);

        assert_eq!(ring.get(&120), vec![node1]); // 840
        assert_eq!(ring.get(&130), vec![node3]); // 910
        assert_eq!(ring.get(&140), vec![node1]); // 980

        assert_eq!(ring.get(&125), vec![node1]); // 875
        assert_eq!(ring.get(&133), vec![node6]); // 931
        assert_eq!(ring.get(&136), vec![node5]); // 952

        assert_eq!(ring.get(&137), vec![node6]); // 959
        assert_eq!(ring.get(&138), vec![node1]); // 966
        assert_eq!(ring.get(&139), vec![node1]); // 973

        // at least each node as a key
        let mut nodes = vec![0; 6];
        for x in 0..50_000 {
            let node = ring.get(&x);
            if node1 == node[0] {
                nodes[0] += 1;
            }
            if node2 == node[0] {
                nodes[1] += 1;
            }
            if node3 == node[0] {
                nodes[2] += 1;
            }
            if node4 == node[0] {
                nodes[3] += 1;
            }
            if node5 == node[0] {
                nodes[4] += 1;
            }
            if node6 == node[0] {
                nodes[5] += 1;
            }
        }
        println!("{nodes:?}",);
        assert!(nodes.iter().all(|x| *x != 0));
    }

    #[test]
    fn get_nodes_with_replicas() {
        let hash_builder = FixedBuildHasher {};

        let mut ring: HashRing<Node, FixedBuildHasher> = HashRing::with_hasher(2, 1, hash_builder);

        assert_eq!(ring.get(&"foo"), vec![]);

        let node1 = Node::new("127.0.0.1"); // hashes 896   
        let node2 = Node::new("127.0.0.2"); // hashes   903     
        let node3 = Node::new("127.0.0.3"); // hashes     910      
        let node4 = Node::new("127.0.0.4"); // hashes       917        
        let node5 = Node::new("127.0.0.5"); // hashes         924        
        let node6 = Node::new("127.0.0.6"); // hashes           931     

        ring.add(node5);
        ring.add(node1);
        ring.add(node3);
        ring.add(node2);
        ring.add(node6);
        ring.add(node4);

        assert_eq!(vec![node3, node4, node5], ring.get(&130));
    }

    #[test]
    fn get_with_replicas_returns_too_many_replicas() {
        let hash_builder = FixedBuildHasher {};

        let mut ring: HashRing<Node, FixedBuildHasher> = HashRing::with_hasher(20, 1, hash_builder);

        assert_eq!(ring.get(&"foo"), vec![]);

        let node1 = Node::new("127.0.0.1"); // hashes 896   
        let node2 = Node::new("127.0.0.2"); // hashes   903     
        let node3 = Node::new("127.0.0.3"); // hashes     910      
        let node4 = Node::new("127.0.0.4"); // hashes       917      
        let node5 = Node::new("127.0.0.5"); // hashes         924         
        let node6 = Node::new("127.0.0.6"); // hashes           931    

        ring.add(node1);
        ring.add(node2);
        ring.add(node3);
        ring.add(node4);
        ring.add(node5);
        ring.add(node6);

        assert_eq!(
            vec![node3, node4, node5, node6, node1, node2],
            ring.get(&130),
            "too high of replicas causes the count to shrink to ring length"
        );
    }

    #[test]
    fn hash_ring_eq() {
        let mut ring: HashRing<Node> = HashRing::new(0, 1);
        let mut other = ring.clone();
        assert_eq!(ring, other);
        assert_eq!(ring.len(), 0);

        let node1 = Node::new("127.0.0.1");
        let node2 = Node::new("127.0.0.2");
        let node3 = Node::new("127.0.0.3");

        other.add(node1);
        other.add(node2);
        other.add(node3);
        assert_ne!(ring, other);
        assert_eq!(other.len(), 3);

        other.remove(&node1);
        other.remove(&node2);
        other.remove(&node3);
        assert_eq!(ring, other);
        assert_eq!(other.len(), 0);

        ring.add(node1);
        ring.add(node2);
        ring.remove(&node1);

        other.add(node2);
        other.add(node3);
        other.remove(&node3);

        assert_eq!(ring.len(), 1);
        assert_eq!(other.len(), 1);
        assert_eq!(ring, other);
    }
}
