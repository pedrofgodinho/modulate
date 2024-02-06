use crate::ModKey;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub(crate) enum Node {
    Dir {
        name: String,
        children: HashMap<String, Node>,
    },
    File {
        name: String,
    },
}

impl Node {
    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name().unwrap().to_str().unwrap();
        if name == "mod.toml" {
            return None;
        }
        Some(if path.is_dir() {
            let children = fs::read_dir(path)
                .unwrap()
                .filter_map(|entry| {
                    let entry = entry.unwrap();
                    let node = Node::from_path(&entry.path());
                    node.map(|node| (node.name().to_string(), node))
                })
                .collect();
            Self::Dir {
                name: name.to_string(),
                children,
            }
        } else {
            Self::File {
                name: name.to_string(),
            }
        })
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Node::Dir { name, .. } => name,
            Node::File { name, .. } => name,
        }
    }
}

#[derive(Debug)]
pub(crate) enum SourcedNode {
    Dir {
        name: String,
        children: HashMap<String, SourcedNode>,
    },
    File {
        name: String,
        source: ModKey,
    },
}

impl SourcedNode {
    pub(crate) fn from_node(node: &Node, source: ModKey) -> Self {
        match node {
            Node::Dir { name, children } => {
                let children = children
                    .iter()
                    .map(|(name, node)| (name.clone(), SourcedNode::from_node(node, source)))
                    .collect();
                Self::Dir {
                    name: name.clone(),
                    children,
                }
            }
            Node::File { name } => Self::File {
                name: name.clone(),
                source,
            },
        }
    }

    pub(crate) fn overwrite_with(&mut self, node: &Node, source: ModKey) {
        match (&mut *self, node) {
            (
                SourcedNode::Dir {
                    name: _, children, ..
                },
                Node::Dir {
                    children: new_children,
                    ..
                },
            ) => {
                for (new_name, new_node) in new_children {
                    let mut found = false;
                    for (name, node) in &mut *children {
                        if name == new_name {
                            found = true;
                            node.overwrite_with(new_node, source);
                            break;
                        }
                    }
                    if !found {
                        children.insert(new_name.clone(), SourcedNode::from_node(new_node, source));
                    }
                }
            }
            (SourcedNode::File { .. }, Node::File { .. }) => {
                *self = SourcedNode::from_node(node, source);
            }
            _ => {}
        }
    }

    pub(crate) fn tree_edit_distance(
        &self,
        new_tree: &SourcedNode,
        ops: &mut Vec<Operation>,
        current_path: &str,
    ) {
        match (self, new_tree) {
            (
                SourcedNode::Dir {
                    name: _,
                    children: old_children,
                    ..
                },
                SourcedNode::Dir {
                    children: new_children,
                    ..
                },
            ) => {
                for (name, node) in old_children {
                    if let Some(new_node) = new_children.get(name) {
                        node.tree_edit_distance(new_node, ops, &format!("{}/{}", current_path, name));
                    } else {
                        match node {
                            SourcedNode::Dir { .. } => {
                                node.ops_for_remove_dir(&format!("{}/{}", current_path, name), ops);
                            }
                            SourcedNode::File { .. } => {
                                ops.push(Operation {
                                    kind: OperationKind::RemoveFile,
                                    path: format!("{}/{}", current_path, name),
                                });
                            }
                        }
                    }
                }
                for (name, new_node) in new_children {
                    if !old_children.contains_key(name) {
                        match new_node {
                            SourcedNode::Dir { .. } => {
                                new_node.ops_for_create_dir(&format!("{}/{}", current_path, name), ops);
                            }
                            SourcedNode::File { source, .. } => {
                                ops.push(Operation {
                                    kind: OperationKind::CreateFile(*source),
                                    path: format!("{}/{}", current_path, name),
                                });
                            }
                        }
                    }
                }
            },
            (
                SourcedNode::File {
                    name: _,
                    source: old_source,
                },
                SourcedNode::File {
                    name: _,
                    source: new_source,
                },
            ) => {
                if old_source != new_source {
                    ops.push(Operation {
                        kind: OperationKind::ChangeSource(*new_source),
                        path: current_path.to_string(),
                    });
                }
            }
            _ => unreachable!("SourcedNode::difference"),
        }
    }

    fn ops_for_create_dir(&self, path: &str, ops: &mut Vec<Operation>) {
        // create dir and all children
        match self {
            SourcedNode::Dir { name: _, children } => {
                ops.push(Operation {
                    kind: OperationKind::CreateDir,
                    path: path.to_string(),
                });
                for (name, node) in children {
                    node.ops_for_create_dir(&format!("{}/{}", path, name), ops);
                }
            }
            SourcedNode::File { name: _, source } => {
                ops.push(Operation {
                    kind: OperationKind::CreateFile(*source),
                    path: path.to_string(),
                });
            }
        }
    }

    fn ops_for_remove_dir(&self, path: &str, ops: &mut Vec<Operation>) {
        // remove dir and all children
        match self {
            SourcedNode::Dir { name: _, children } => {
                for (name, node) in children {
                    node.ops_for_remove_dir(&format!("{}/{}", path, name), ops);
                }
                ops.push(Operation {
                    kind: OperationKind::RemoveDir,
                    path: path.to_string(),
                });
            }
            SourcedNode::File { name: _, source: _ } => {
                ops.push(Operation {
                    kind: OperationKind::RemoveFile,
                    path: path.to_string(),
                });
            }
        }
    }

    pub(crate) fn print(&self, ident: usize) {
        match self {
            SourcedNode::Dir { name, children } => {
                println!("{}{}", "  ".repeat(ident), name);
                for (_, node) in children {
                    node.print(ident + 1);
                }
            }
            SourcedNode::File { name, source } => {
                println!("{}{}: {:?}", "  ".repeat(ident), name, source);
            }
        }
    }
}

impl Display for SourcedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourcedNode::Dir { name, children } => {
                write!(f, "{}", name)?;
                for (name, node) in children {
                    write!(f, "\n{}", node)?;
                }
                Ok(())
            }
            SourcedNode::File { name, source } => write!(f, "{}: {:?}", name, source),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Operation {
    pub(crate) kind: OperationKind,
    pub(crate) path: String,
}

#[derive(Debug)]
pub(crate) enum OperationKind {
    CreateDir,
    RemoveDir,
    CreateFile(ModKey),
    RemoveFile,
    ChangeSource(ModKey),
}
