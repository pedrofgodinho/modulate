use crate::node::Node;
use crate::ModError;
use semver::Version;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Mod {
    pub(crate) metadata: ModMetadata,
    pub(crate) dir: PathBuf,
    pub(crate) node: Node,
}

impl Mod {
    pub(crate) fn new(dir: PathBuf) -> Result<Self, ModError> {
        if !Path::new(&dir).is_dir() {
            return Err(ModError::DirNotFound(dir.to_string_lossy().to_string()));
        }
        let dir = fs::canonicalize(dir).unwrap();
        // read metadata
        let metadata_path = dir.join("mod.toml");
        if !metadata_path.exists() {
            return Err(ModError::ModMetadataMissing(dir.to_string_lossy().to_string()));
        }
        let metadata =
            toml::from_str::<ModMetadata>(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        Ok(Self {
            metadata,
            node: Node::from_path(&dir).unwrap(),
            dir,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModMetadata {
    pub name: String,
    pub version: Version,
    pub uuid: Uuid,
}
