//! Knowledge Graph and Mind Map generation
//!
//! This module handles the automatic discovery and visualization of
//! relationships between memories - creating mind maps, project graphs,
//! and connection networks.

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// A mind map visualization centered on a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMap {
    pub center_id: MemoryId,
    pub nodes: Vec<MindMapNode>,
    pub edges: Vec<MindMapEdge>,
    pub layout: LayoutInfo,
    pub stats: MindMapStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMapNode {
    pub id: MemoryId,
    pub label: String,
    pub kind: String,
    pub depth: usize,
    pub is_center: bool,
    pub color: String,
    pub size: f32,
    pub metadata: NodeMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub path: String,
    pub tags: Vec<String>,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMapEdge {
    pub source: MemoryId,
    pub target: MemoryId,
    pub kind: String,
    pub strength: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutInfo {
    pub algorithm: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMapStats {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub connection_types: HashMap<String, usize>,
}

/// Code-specific graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    pub root: String,
    pub nodes: Vec<CodeGraphNode>,
    pub edges: Vec<CodeGraphEdge>,
    pub modules: Vec<ModuleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraphNode {
    pub id: MemoryId,
    pub path: String,
    pub language: String,
    pub lines: u32,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub functions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraphEdge {
    pub source: MemoryId,
    pub target: MemoryId,
    pub kind: CodeEdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeEdgeKind {
    Imports,
    Exports,
    References,
    Tests,
    SameModule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub file_count: usize,
    pub total_lines: u32,
}

/// The knowledge graph that connects all memories
pub struct KnowledgeGraph {
    storage: Arc<Storage>,
}

impl KnowledgeGraph {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Build a mind map centered on a specific memory
    pub async fn build_mind_map(&self, center_id: MemoryId, depth: usize) -> Result<MindMap> {
        let _center = self
            .storage
            .get_memory(center_id)
            .await?
            .ok_or_else(|| HippoError::NotFound(format!("Memory {} not found", center_id)))?;

        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(MemoryId, usize)> = VecDeque::new();

        // Start BFS from center
        queue.push_back((center_id, 0));
        visited.insert(center_id);

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth > depth {
                continue;
            }

            let memory = match self.storage.get_memory(current_id).await? {
                Some(m) => m,
                None => continue,
            };

            // Add node
            nodes.insert(
                current_id,
                MindMapNode {
                    id: current_id,
                    label: memory.metadata.title.clone().unwrap_or_else(|| {
                        memory
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string()
                    }),
                    kind: memory_kind_to_string(&memory.kind),
                    depth: current_depth,
                    is_center: current_id == center_id,
                    color: kind_to_color(&memory.kind),
                    size: if current_id == center_id {
                        1.5
                    } else {
                        1.0 - (current_depth as f32 * 0.15)
                    }
                    .max(0.4),
                    metadata: NodeMetadata {
                        path: memory.path.to_string_lossy().to_string(),
                        tags: memory.tags.iter().map(|t| t.name.clone()).collect(),
                        date: memory.created_at.to_rfc3339(),
                    },
                    position: None, // Computed later
                },
            );

            // Process explicit connections
            for conn in &memory.connections {
                if !visited.contains(&conn.target_id) && current_depth < depth {
                    visited.insert(conn.target_id);
                    queue.push_back((conn.target_id, current_depth + 1));
                }

                edges.push(MindMapEdge {
                    source: current_id,
                    target: conn.target_id,
                    kind: connection_kind_to_string(&conn.kind),
                    strength: conn.strength,
                    label: connection_kind_label(&conn.kind),
                });
            }

            // Find similar memories via vector search (shallow depth only)
            if current_depth < 2 {
                let similar = self
                    .storage
                    .find_similar(current_id, 5)
                    .await
                    .unwrap_or_default();
                for (similar_id, score) in similar {
                    if similar_id != current_id
                        && !visited.contains(&similar_id)
                        && current_depth < depth
                    {
                        visited.insert(similar_id);
                        queue.push_back((similar_id, current_depth + 1));
                    }

                    if similar_id != current_id && score > 0.7 {
                        edges.push(MindMapEdge {
                            source: current_id,
                            target: similar_id,
                            kind: "similar".to_string(),
                            strength: score,
                            label: Some(format!("{:.0}% similar", score * 100.0)),
                        });
                    }
                }
            }
        }

        // Compute radial layout positions
        let mut nodes_vec: Vec<MindMapNode> = nodes.into_values().collect();
        compute_radial_layout(&mut nodes_vec, center_id);

        let connection_types = count_connection_types(&edges);

        Ok(MindMap {
            center_id,
            nodes: nodes_vec,
            edges,
            layout: LayoutInfo {
                algorithm: "radial".to_string(),
                width: 800.0,
                height: 800.0,
            },
            stats: MindMapStats {
                total_nodes: visited.len(),
                max_depth: depth,
                connection_types,
            },
        })
    }

    /// Get related memories
    pub async fn get_related(&self, memory_id: MemoryId, limit: usize) -> Result<Vec<Memory>> {
        let memory = self
            .storage
            .get_memory(memory_id)
            .await?
            .ok_or_else(|| HippoError::NotFound(format!("Memory {} not found", memory_id)))?;

        let mut related_ids: Vec<(MemoryId, f32)> = Vec::new();

        // From explicit connections
        for conn in &memory.connections {
            related_ids.push((conn.target_id, conn.strength));
        }

        // From vector similarity
        let similar = self
            .storage
            .find_similar(memory_id, limit * 2)
            .await
            .unwrap_or_default();
        for (id, score) in similar {
            if id != memory_id && !related_ids.iter().any(|(rid, _)| *rid == id) {
                related_ids.push((id, score));
            }
        }

        // Sort by score and take top N
        related_ids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        related_ids.truncate(limit);

        // Fetch memories
        let mut memories = Vec::new();
        for (id, _) in related_ids {
            if let Some(m) = self.storage.get_memory(id).await? {
                memories.push(m);
            }
        }

        Ok(memories)
    }

    /// Build a code dependency graph for a project
    pub async fn build_code_graph(&self, root_path: &str) -> Result<CodeGraph> {
        let memories = self.storage.find_by_path_prefix(root_path).await?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut file_to_id: HashMap<String, MemoryId> = HashMap::new();
        let mut modules: HashMap<String, ModuleInfo> = HashMap::new();

        // First pass: create nodes and track modules
        for memory in &memories {
            if let MemoryKind::Code { language, lines } = &memory.kind {
                let path_str = memory.path.to_string_lossy().to_string();
                file_to_id.insert(path_str.clone(), memory.id);

                // Track module (directory)
                if let Some(parent) = memory.path.parent() {
                    let module_path = parent.to_string_lossy().to_string();
                    let module_name = parent
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("root")
                        .to_string();

                    modules
                        .entry(module_path.clone())
                        .and_modify(|m| {
                            m.file_count += 1;
                            m.total_lines += lines;
                        })
                        .or_insert(ModuleInfo {
                            name: module_name,
                            path: module_path,
                            file_count: 1,
                            total_lines: *lines,
                        });
                }

                nodes.push(CodeGraphNode {
                    id: memory.id,
                    path: path_str,
                    language: language.clone(),
                    lines: *lines,
                    imports: memory
                        .metadata
                        .code_info
                        .as_ref()
                        .map(|c| c.imports.clone())
                        .unwrap_or_default(),
                    exports: memory
                        .metadata
                        .code_info
                        .as_ref()
                        .map(|c| c.exports.clone())
                        .unwrap_or_default(),
                    functions: memory
                        .metadata
                        .code_info
                        .as_ref()
                        .map(|c| c.functions.len())
                        .unwrap_or(0),
                });
            }
        }

        // Second pass: create edges based on imports
        for node in &nodes {
            if let Some(memory) = memories.iter().find(|m| m.id == node.id) {
                if let Some(code_info) = &memory.metadata.code_info {
                    for import in &code_info.imports {
                        // Try to resolve import to a file in our graph
                        if let Some(target_id) =
                            resolve_import(&import, &file_to_id, &node.path, &node.language)
                        {
                            if target_id != node.id {
                                edges.push(CodeGraphEdge {
                                    source: node.id,
                                    target: target_id,
                                    kind: CodeEdgeKind::Imports,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Add same-module edges
        let files_by_module: HashMap<String, Vec<MemoryId>> = nodes
            .iter()
            .filter_map(|n| {
                std::path::Path::new(&n.path)
                    .parent()
                    .map(|p| (p.to_string_lossy().to_string(), n.id))
            })
            .fold(HashMap::new(), |mut acc, (module, id)| {
                acc.entry(module).or_default().push(id);
                acc
            });

        for (_, file_ids) in files_by_module {
            if file_ids.len() > 1 && file_ids.len() <= 10 {
                for i in 0..file_ids.len() {
                    for j in (i + 1)..file_ids.len() {
                        edges.push(CodeGraphEdge {
                            source: file_ids[i],
                            target: file_ids[j],
                            kind: CodeEdgeKind::SameModule,
                        });
                    }
                }
            }
        }

        Ok(CodeGraph {
            root: root_path.to_string(),
            nodes,
            edges,
            modules: modules.into_values().collect(),
        })
    }

    /// Auto-detect clusters of related memories
    pub async fn detect_clusters(&self) -> Result<Vec<Cluster>> {
        // This would use clustering algorithms on the vector embeddings
        // For now, return clusters based on folder structure and dates
        self.storage.list_clusters().await
    }

    /// Suggest connections for a memory based on content similarity
    pub async fn suggest_connections(
        &self,
        memory_id: MemoryId,
    ) -> Result<Vec<(Memory, ConnectionKind, f32)>> {
        let similar = self.storage.find_similar(memory_id, 20).await?;
        let mut suggestions = Vec::new();

        for (id, score) in similar {
            if id == memory_id || score < 0.5 {
                continue;
            }

            if let Some(memory) = self.storage.get_memory(id).await? {
                let kind = infer_connection_kind(&memory, score);
                suggestions.push((memory, kind, score));
            }
        }

        Ok(suggestions)
    }
}

// Helper functions

fn memory_kind_to_string(kind: &MemoryKind) -> String {
    match kind {
        MemoryKind::Image { .. } => "image",
        MemoryKind::Video { .. } => "video",
        MemoryKind::Audio { .. } => "audio",
        MemoryKind::Document { .. } => "document",
        MemoryKind::Spreadsheet { .. } => "spreadsheet",
        MemoryKind::Presentation { .. } => "presentation",
        MemoryKind::Code { .. } => "code",
        MemoryKind::Archive { .. } => "archive",
        MemoryKind::Database => "database",
        MemoryKind::Folder => "folder",
        MemoryKind::Unknown => "unknown",
    }
    .to_string()
}

fn kind_to_color(kind: &MemoryKind) -> String {
    match kind {
        MemoryKind::Image { .. } => "#E8DDD4",        // Warm beige
        MemoryKind::Video { .. } => "#D4DDE8",        // Cool blue
        MemoryKind::Audio { .. } => "#E8D4DD",        // Soft pink
        MemoryKind::Document { .. } => "#DDE8D4",     // Soft green
        MemoryKind::Code { .. } => "#DDD4E8",         // Soft purple
        MemoryKind::Spreadsheet { .. } => "#D4E8DD",  // Mint
        MemoryKind::Presentation { .. } => "#E8E4D4", // Cream
        _ => "#E0E0E0",                               // Gray
    }
    .to_string()
}

fn connection_kind_to_string(kind: &ConnectionKind) -> String {
    match kind {
        ConnectionKind::SameFolder => "same_folder",
        ConnectionKind::SameAlbum => "same_album",
        ConnectionKind::LinkedInDocument => "linked",
        ConnectionKind::Imports => "imports",
        ConnectionKind::ImportedBy => "imported_by",
        ConnectionKind::References => "references",
        ConnectionKind::SimilarContent => "similar",
        ConnectionKind::SameEvent => "same_event",
        ConnectionKind::SamePerson => "same_person",
        ConnectionKind::SameProject => "same_project",
        ConnectionKind::Custom(s) => s,
    }
    .to_string()
}

fn connection_kind_label(kind: &ConnectionKind) -> Option<String> {
    match kind {
        ConnectionKind::Imports => Some("imports".to_string()),
        ConnectionKind::ImportedBy => Some("used by".to_string()),
        ConnectionKind::SameEvent => Some("same event".to_string()),
        ConnectionKind::SamePerson => Some("same person".to_string()),
        _ => None,
    }
}

fn count_connection_types(edges: &[MindMapEdge]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for edge in edges {
        *counts.entry(edge.kind.clone()).or_insert(0) += 1;
    }
    counts
}

fn compute_radial_layout(nodes: &mut [MindMapNode], center_id: MemoryId) {
    let center_x = 400.0;
    let center_y = 400.0;
    let radius_step = 120.0;

    // Group by depth
    let mut by_depth: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        by_depth.entry(node.depth).or_default().push(i);
    }

    // Position center node
    for node in nodes.iter_mut() {
        if node.id == center_id {
            node.position = Some(Position {
                x: center_x,
                y: center_y,
            });
            break;
        }
    }

    // Position nodes at each depth in a circle
    for (depth, indices) in by_depth.iter() {
        if *depth == 0 {
            continue;
        }

        let radius = *depth as f32 * radius_step;
        let count = indices.len();
        let angle_step = 2.0 * std::f32::consts::PI / count as f32;

        for (i, &idx) in indices.iter().enumerate() {
            let angle = i as f32 * angle_step - std::f32::consts::PI / 2.0;
            nodes[idx].position = Some(Position {
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
            });
        }
    }
}

fn resolve_import(
    import: &str,
    file_to_id: &HashMap<String, MemoryId>,
    current_path: &str,
    language: &str,
) -> Option<MemoryId> {
    // Simple resolution - in reality would need proper module resolution
    let current_dir = std::path::Path::new(current_path).parent()?;

    // Handle relative imports
    if import.starts_with('.') || import.starts_with("./") || import.starts_with("../") {
        let clean_import = import.trim_start_matches("./");
        let resolved = current_dir.join(clean_import);

        // Try with common extensions
        for ext in &["", ".rs", ".py", ".js", ".ts", ".tsx", ".jsx"] {
            let with_ext = format!("{}{}", resolved.to_string_lossy(), ext);
            if let Some(&id) = file_to_id.get(&with_ext) {
                return Some(id);
            }
        }
    }

    // Handle Rust module imports (e.g., "crate::module::submodule")
    if language == "rust" && import.starts_with("crate::") {
        let module_path = import.strip_prefix("crate::")?.replace("::", "/");
        for ext in &[".rs", "/mod.rs"] {
            let full_path = format!("{}/{}{}", current_dir.to_string_lossy(), module_path, ext);
            if let Some(&id) = file_to_id.get(&full_path) {
                return Some(id);
            }
        }
    }

    None
}

fn infer_connection_kind(memory: &Memory, score: f32) -> ConnectionKind {
    if score > 0.9 {
        ConnectionKind::SimilarContent
    } else if memory.metadata.location.is_some() {
        ConnectionKind::SameEvent
    } else {
        ConnectionKind::SimilarContent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_to_color() {
        let img = MemoryKind::Image {
            width: 100,
            height: 100,
            format: "jpg".into(),
        };
        assert!(!kind_to_color(&img).is_empty());
    }

    #[test]
    fn test_radial_layout() {
        let mut nodes = vec![
            MindMapNode {
                id: uuid::Uuid::new_v4(),
                label: "Center".into(),
                kind: "document".into(),
                depth: 0,
                is_center: true,
                color: "#fff".into(),
                size: 1.5,
                metadata: NodeMetadata {
                    path: "/test".into(),
                    tags: vec![],
                    date: "2024-01-01".into(),
                },
                position: None,
            },
            MindMapNode {
                id: uuid::Uuid::new_v4(),
                label: "Child 1".into(),
                kind: "document".into(),
                depth: 1,
                is_center: false,
                color: "#fff".into(),
                size: 1.0,
                metadata: NodeMetadata {
                    path: "/test/child1".into(),
                    tags: vec![],
                    date: "2024-01-01".into(),
                },
                position: None,
            },
        ];

        let center_id = nodes[0].id;
        compute_radial_layout(&mut nodes, center_id);

        assert!(nodes[0].position.is_some());
        assert!(nodes[1].position.is_some());
    }
}
