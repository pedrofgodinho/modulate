use crate::node::Node;
use crate::ModError;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
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
        // check if serialized mod exists
        let bin_path = dir.join("mod.bin");
        if bin_path.exists() {
            let file = fs::File::open(bin_path).unwrap();
            return Ok(bincode::deserialize_from(file).unwrap());
        }

        // read metadata
        let metadata_path = dir.join("mod.toml");
        if !metadata_path.exists() {
            return Err(ModError::ModMetadataMissing(dir.to_string_lossy().to_string()));
        }
        let metadata =
            toml::from_str::<ModMetadata>(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        let r =
        Self {
            metadata,
            node: Node::from_path(&dir).unwrap(),
            dir,
        };
        let file = fs::File::create(bin_path).unwrap();
        bincode::serialize_into(file, &r).unwrap();

        Ok(r)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModMetadata {
    pub name: String,
    pub version: Version,
    pub uuid: Uuid,
}
