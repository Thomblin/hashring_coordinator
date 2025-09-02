# hashring_coordinator

[<img alt="github" src="https://img.shields.io/badge/github-thomblin/hashring_coordinator-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/thomblin/hashring_coordinator)
[<img alt="crates.io" src="https://img.shields.io/crates/v/hashring_coordinator?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/hashring_coordinator)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/hashring_coordinator?logo=docs.rs&labelColor=555555" height="20">](https://docs.rs/hashring_coordinator)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/Thomblin/hashring_coordinator/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/thomblin/hashring_coordinator/actions?query=branch%3Amain)
[<img alt="audit status" src="https://img.shields.io/github/actions/workflow/status/Thomblin/hashring_coordinator/audit.yml?branch=main&style=for-the-badge&label=audit" height="20">](https://github.com/thomblin/hashring_coordinator/actions?query=branch%3Amain)

A minimal implementation of consistent hashing.

Clients can use the `HashRing` struct to add consistent hashing to their
applications. 

- You can add and remove nodes to a HashRing.
- Find all nodes that (should) store a given key.
- Return all hash ranges within the HashRing to easily detect nodes and their responsibilities (containing replica nodes as well)
- Compare two HashRing clusters to receive replication instructions between both clusters (for each node, list hash ranges and target nodes to find keys that need to be replicated)

This implemementation is based on the original source: <https://github.com/jeromefroe/hashring-rs>

## Example

Take a look at the examples directory for further details like replication during deployments or after adding/removing nodes from the cluster

* [`simple.rs`](/examples/simple.rs) - gives a brief overview of the main functions
* [`cluster.rs`](/examples/cluster.rs) - implements a cluster and shows how to rebalance the cluster if a node as added or removed and how to synchronize all values to a completely new cluster