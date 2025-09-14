#[cfg(feature = "derive")]
#[cfg(test)]
mod tests {
    use hashring_coordinator::Replicas;

    #[test]
    fn test_serialize_and_deserialize_replicas() {
        let original = Replicas {
            hash_range: 0..=10,
            nodes: vec!["node1", "node2", "node3"],
        };

        // Serialize the `Replicas` instance to JSON
        let serialized = serde_json::to_string(&original).expect("Serialization failed");

        // Deserialize the JSON string back into a `Replicas` instance
        let deserialized: Replicas<&str> =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        // Assert that the original and deserialized instances are equal
        assert_eq!(original, deserialized);
    }
}
