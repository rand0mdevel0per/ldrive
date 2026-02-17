use ldrive_common::NodeId;
use std::collections::BTreeMap;
use std::net::SocketAddr;

/// Number of vnodes per physical node
pub const VNODES_PER_NODE: u32 = 128;

/// Virtual node identifier: hash of (NodeId + vnode_index)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VnodeId(pub [u8; 20]);

impl VnodeId {
    /// Create vnode ID from physical node ID and vnode index
    pub fn new(node_id: &NodeId, vnode_index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&node_id.0);
        hasher.update(&vnode_index.to_le_bytes());
        let hash = hasher.finalize();
        let mut id = [0u8; 20];
        id.copy_from_slice(&hash.as_bytes()[..20]);
        Self(id)
    }

    /// Create vnode ID from content hash (for lookup)
    pub fn from_content(content_hash: &[u8]) -> Self {
        let hash = blake3::hash(content_hash);
        let mut id = [0u8; 20];
        id.copy_from_slice(&hash.as_bytes()[..20]);
        Self(id)
    }
}

/// Physical node info tracked by vnode ring
#[derive(Debug, Clone)]
pub struct PhysicalNode {
    pub node_id: NodeId,
    pub addr: SocketAddr,
    pub region: String,
}

/// Consistent hash ring with vnodes
pub struct VnodeRing {
    /// Map from vnode ID to physical node
    ring: BTreeMap<VnodeId, PhysicalNode>,
}

impl VnodeRing {
    pub fn new() -> Self {
        Self {
            ring: BTreeMap::new(),
        }
    }

    /// Add a physical node with its vnodes to the ring
    pub fn add_node(&mut self, node: PhysicalNode) {
        for i in 0..VNODES_PER_NODE {
            let vnode_id = VnodeId::new(&node.node_id, i);
            self.ring.insert(vnode_id, node.clone());
        }
    }

    /// Remove a physical node and all its vnodes
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.ring.retain(|_, node| node.node_id != *node_id);
    }

    /// Find N closest vnodes (physical nodes) to a content hash
    pub fn find_nodes(&self, content_hash: &[u8], count: usize) -> Vec<PhysicalNode> {
        if self.ring.is_empty() {
            return vec![];
        }

        let target = VnodeId::from_content(content_hash);
        let mut results = Vec::new();
        let mut seen_nodes = std::collections::HashSet::new();

        // Start from target position and walk clockwise
        for (_, node) in self.ring.range(target..) {
            if seen_nodes.insert(node.node_id) {
                results.push(node.clone());
                if results.len() >= count {
                    return results;
                }
            }
        }

        // Wrap around to beginning
        for (_, node) in self.ring.iter() {
            if seen_nodes.insert(node.node_id) {
                results.push(node.clone());
                if results.len() >= count {
                    return results;
                }
            }
        }

        results
    }

    /// Find nodes with region affinity: prefer in-region, then out-of-region
    pub fn find_nodes_with_affinity(
        &self,
        content_hash: &[u8],
        local_region: &str,
        in_region_count: usize,
        out_region_count: usize,
    ) -> (Vec<PhysicalNode>, Vec<PhysicalNode>) {
        // Disable region affinity if network has < 10 nodes
        if self.node_count() < 10 {
            let nodes = self.find_nodes(content_hash, in_region_count + out_region_count);
            return (nodes, vec![]);
        }

        let candidates = self.find_nodes(content_hash, (in_region_count + out_region_count) * 3);

        let mut in_region = Vec::new();
        let mut out_region = Vec::new();

        for node in candidates {
            if node.region == local_region && in_region.len() < in_region_count {
                in_region.push(node);
            } else if node.region != local_region && out_region.len() < out_region_count {
                out_region.push(node);
            }

            if in_region.len() >= in_region_count && out_region.len() >= out_region_count {
                break;
            }
        }

        (in_region, out_region)
    }

    pub fn node_count(&self) -> usize {
        self.ring.values().map(|n| n.node_id).collect::<std::collections::HashSet<_>>().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vnode_distribution() {
        let mut ring = VnodeRing::new();

        let node1 = PhysicalNode {
            node_id: NodeId([1u8; 20]),
            addr: "127.0.0.1:8001".parse().unwrap(),
            region: "cn-east".to_string(),
        };
        let node2 = PhysicalNode {
            node_id: NodeId([2u8; 20]),
            addr: "127.0.0.1:8002".parse().unwrap(),
            region: "us-west".to_string(),
        };

        ring.add_node(node1);
        ring.add_node(node2);

        assert_eq!(ring.node_count(), 2);

        let nodes = ring.find_nodes(b"test_content", 2);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn region_affinity() {
        let mut ring = VnodeRing::new();

        for i in 0..6 {
            let region = if i < 3 { "cn-east" } else { "us-west" };
            ring.add_node(PhysicalNode {
                node_id: NodeId([i; 20]),
                addr: format!("127.0.0.1:800{}", i).parse().unwrap(),
                region: region.to_string(),
            });
        }

        let (in_region, out_region) = ring.find_nodes_with_affinity(b"test", "cn-east", 3, 3);
        assert_eq!(in_region.len(), 3);
        assert_eq!(out_region.len(), 3);
        assert!(in_region.iter().all(|n| n.region == "cn-east"));
        assert!(out_region.iter().all(|n| n.region != "cn-east"));
    }
}
