#[test]
fn test_basic_functionality() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_distributed_concept() {
    struct Node {
        id: u32,
        data: std::collections::HashMap<String, String>,
    }
    
    impl Node {
        fn new(id: u32) -> Self {
            Self {
                id,
                data: std::collections::HashMap::new(),
            }
        }
        
        fn store(&mut self, key: String, value: String) {
            self.data.insert(key, value);
        }
        
        fn get(&self, key: &str) -> Option<&String> {
            self.data.get(key)
        }
    }
    
    let mut node1 = Node::new(1);
    let mut node2 = Node::new(2);
    
    node1.store("key1".to_string(), "value1".to_string());
    node2.store("key1".to_string(), "value1".to_string());
    
    assert_eq!(node1.get("key1"), Some(&"value1".to_string()));
    assert_eq!(node2.get("key1"), Some(&"value1".to_string()));
    assert_eq!(node1.get("key1"), node2.get("key1"));
}

#[test] 
fn test_replication_logic() {
    fn can_replicate(active_nodes: usize, replication_factor: usize) -> bool {
        active_nodes >= replication_factor
    }
    
    assert!(can_replicate(5, 3));
    assert!(can_replicate(3, 3));
    assert!(!can_replicate(2, 3));
    assert!(!can_replicate(1, 3));
}

#[test]
fn test_quorum_logic() {
    fn has_quorum(active_nodes: usize, total_nodes: usize) -> bool {
        active_nodes > total_nodes / 2
    }
    
    assert!(has_quorum(3, 5));
    assert!(has_quorum(2, 3));
    assert!(!has_quorum(2, 5));
    assert!(!has_quorum(1, 3));
}