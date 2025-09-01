use super::{HashRing, Node};

pub struct HashRingIterator<T> {
    ring: std::vec::IntoIter<Node<T>>,
}

impl<T> Iterator for HashRingIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.ring.next().map(|node| node.node)
    }
}

impl<T> IntoIterator for HashRing<T> {
    type Item = T;

    type IntoIter = HashRingIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        HashRingIterator {
            ring: self.ring.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::str::FromStr;

    use super::HashRing;

    #[derive(Debug, Copy, Clone, PartialEq)]
    struct VNode {
        id: usize,
        addr: SocketAddrV4,
    }

    impl VNode {
        fn new(ip: &str, port: u16, id: usize) -> Self {
            let addr = SocketAddrV4::new(Ipv4Addr::from_str(ip).unwrap(), port);
            VNode { id, addr }
        }
    }

    impl Hash for VNode {
        fn hash<H: Hasher>(&self, s: &mut H) {
            (self.id, self.addr.port(), self.addr.ip()).hash(s)
        }
    }

    #[test]
    fn into_iter() {
        let mut ring: HashRing<VNode> = HashRing::new(0, 1);

        assert_eq!(ring.get(&"foo"), vec![]);

        let vnode1 = VNode::new("127.0.0.1", 1024, 1);
        let vnode2 = VNode::new("127.0.0.1", 1024, 2);
        let vnode3 = VNode::new("127.0.0.2", 1024, 1);

        ring.add(vnode1);
        ring.add(vnode2);
        ring.add(vnode3);

        let mut iter = ring.into_iter();

        assert_eq!(Some(vnode1), iter.next());
        assert_eq!(Some(vnode3), iter.next());
        assert_eq!(Some(vnode2), iter.next());
        assert_eq!(None, iter.next());
    }
}
