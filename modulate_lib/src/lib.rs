pub mod r#mod;
mod node;

use crate::node::{Operation, OperationKind, SourcedNode};
use crate::r#mod::{Mod, ModMetadata};
use log::{error, info, trace};
use slotmap::{new_key_type, SlotMap};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ModError {
    #[error("Dir not found: {0}")]
    DirNotFound(String),
    #[error("Invalid mod index: {0}")]
    InvalidModUuid(Uuid),
    #[error("Invalid mod order: {0:?}")]
    InvalidModOrder(Vec<usize>),
    #[error("Mod metadata missing: {0}")]
    ModMetadataMissing(String),
    #[error("Invalid mod metadata: {0}")]
    InvalidModMetadata(String),
    #[error("Couldn't create bak dir: {0}")]
    BakDirCreationFailed(String),
}

new_key_type! {
    pub struct ModKey;
}

#[derive(Debug)]
pub struct ModManager {
    working_dir: PathBuf,
    bak_dir: PathBuf,
    active_mods: Vec<ModKey>,
    inactive_mods: Vec<ModKey>,
    hash_map: HashMap<Uuid, ModKey>,
    current_active_tree: SourcedNode,
    slotmap: SlotMap<ModKey, Mod>,
}

impl ModManager {
    /// Create a new ModManager with the given working directory.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// ```
    pub fn new(working_dir: PathBuf, bak_dir: PathBuf) -> Result<Self, ModError> {
        // check if working_dir exists
        if !working_dir.exists() {
            return Err(ModError::DirNotFound(working_dir.to_string_lossy().to_string()));
        }
        fs::create_dir_all(&bak_dir).map_err(|e| {
            error!("Failed to create backup directory");
            ModError::BakDirCreationFailed(e.to_string())
        })?;
        let working_dir = working_dir.canonicalize().unwrap();
        let bak_dir = bak_dir.canonicalize().unwrap();
        Ok(Self {
            working_dir,
            bak_dir,
            active_mods: Vec::new(),
            inactive_mods: Vec::new(),
            hash_map: HashMap::new(),
            current_active_tree: SourcedNode::Dir {
                name: "root".to_string(),
                children: HashMap::new(),
            },
            slotmap: SlotMap::with_key(),
        })
    }

    /// Get a list of active mods.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// manager.add_mod("mod2", "./mod2".into()).unwrap();
    /// manager.activate_mod(0).unwrap();
    /// println!("{:#?}", manager.active_mods());
    /// ```
    pub fn active_mods(&self) -> Vec<&ModMetadata> {
        self.active_mods
            .iter()
            .map(|&key| &self.slotmap[key].metadata)
            .collect()
    }

    /// Get a list of inactive mods.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// manager.add_mod("mod2", "./mod2".into()).unwrap();
    /// println!("{:#?}", manager.inactive_mods());
    /// ```
    pub fn inactive_mods(&self) -> Vec<&ModMetadata> {
        self.inactive_mods
            .iter()
            .map(|&key| &self.slotmap[key].metadata)
            .collect()
    }

    /// Add a mod to the manager. The mod will be inactive by default. Returns the uuid of the mod.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// ```
    pub fn add_mod(&mut self, dir: PathBuf) -> Result<Uuid, ModError> {
        let key = self.slotmap.insert(Mod::new(dir)?);
        self.inactive_mods.push(key);
        self.hash_map.insert(self.slotmap[key].metadata.uuid, key);
        info!("Added mod: {:#?}", self.slotmap[key].metadata.name);
        Ok(self.slotmap[key].metadata.uuid)
    }

    /// Remove a mod by uuid. The mod must be inactive.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// let r#mod = manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// manager.remove_mod(r#mod).unwrap();
    /// ```
    pub fn remove_mod(&mut self, uuid: Uuid) -> Result<(), ModError> {
        if let Some(key) = self.hash_map.get(&uuid) {
            if self.active_mods.contains(key) {
                return Err(ModError::InvalidModUuid(uuid));
            }
            self.slotmap.remove(*key);
            self.hash_map.remove(&uuid);
            info!("Removed mod: {:#?}", uuid);
            Ok(())
        } else {
            Err(ModError::InvalidModUuid(uuid))
        }
    }

    /// Activate a mod by uuid. The mod must be inactive.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// let r#mod = manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// manager.activate_mod(r#mod).unwrap();
    /// ```
    pub fn activate_mod(&mut self, uuid: Uuid) -> Result<(), ModError> {
        if let Some(key) = self.hash_map.get(&uuid) {
            if self.active_mods.contains(key) {
                return Err(ModError::InvalidModUuid(uuid));
            }
            self.inactive_mods.retain(|&k| k != *key);
            self.active_mods.push(*key);
            info!("Activated mod: {:#?}", self.slotmap[*key].metadata.name);
            Ok(())
        } else {
            Err(ModError::InvalidModUuid(uuid))
        }
    }

    /// Deactivate a mod by uuid. The mod must be active.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// let r#mod = manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// manager.activate_mod(r#mod).unwrap();
    /// manager.deactivate_mod(r#mod).unwrap();
    /// ```
    pub fn deactivate_mod(&mut self, uuid: Uuid) -> Result<(), ModError> {
        if let Some(key) = self.hash_map.get(&uuid) {
            if self.inactive_mods.contains(key) {
                return Err(ModError::InvalidModUuid(uuid));
            }
            self.active_mods.retain(|&k| k != *key);
            self.inactive_mods.push(*key);
            info!("Deactivated mod: {:#?}", self.slotmap[*key].metadata.name);
            Ok(())
        } else {
            Err(ModError::InvalidModUuid(uuid))
        }
    }

    // TODO reorder by uuid instead of index
    /// Reorder the active mods by index. The order must contain all active mods.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// let mod1 = manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// let mod2 = manager.add_mod("mod2", "./mod2".into()).unwrap();
    /// manager.activate_mod(mod1).unwrap();
    /// manager.activate_mod(mod2).unwrap();
    /// manager.reorder_mods(&[1, 0]).unwrap();
    /// ```
    pub fn reorder_mods(&mut self, order: &[usize]) -> Result<(), ModError> {
        // TODO optimize check
        if order.len() != self.active_mods.len() || (0..order.len()).any(|i| !order.contains(&i)) {
            return Err(ModError::InvalidModOrder(order.to_vec()));
        }
        let mut new_active_mods = Vec::new();
        for i in order {
            new_active_mods.push(self.active_mods[*i]);
        }
        self.active_mods = new_active_mods;
        Ok(())
    }

    /// Deploy the mods to the working directory.
    ///
    /// Changes made by adding, removing, or reordering mods will not be applied until this method is called.
    ///
    /// # Examples
    /// ```
    /// use modulate_lib::ModManager;
    /// let mut manager = ModManager::new("./working_dir".parse().unwrap(), "./bak".parse().unwrap());
    /// let mod1 = manager.add_mod("mod1", "./mod1".into()).unwrap();
    /// let mod2 = manager.add_mod("mod2", "./mod2".into()).unwrap();
    /// manager.activate_mod(mod1).unwrap();
    /// manager.activate_mod(mod2).unwrap();
    /// manager.reorder_mods(&[1, 0]).unwrap();
    /// manager.apply_mods();
    /// ```
    pub fn deploy_mods(&mut self) {
        let new_tree = self.make_tree();
        let mut ops = Vec::new();
        self.current_active_tree.tree_edit_distance(&new_tree, &mut ops, "");
        self.apply_operations(ops);
        self.current_active_tree = new_tree;
    }

    fn make_tree(&self) -> SourcedNode {
        let mut tree = SourcedNode::Dir {
            name: "root".to_string(),
            children: HashMap::new(),
        };
        info!("Calculating virtual tree");
        for key in self.active_mods.iter().rev() {
            trace!(" - Adding mod: {}", self.slotmap[*key].metadata.name);
            let mod_node = &self.slotmap[*key].node;
            tree.overwrite_with(mod_node, *key);
        }
        tree
    }

    // TODO remove unwraps and handle/return errors
    fn apply_operations(&mut self, ops: Vec<Operation>) {
        for op in ops {
            let path = &op.path[1..];
            let working_file = self.working_dir.join(path);
            let back_file = self.bak_dir.join(path);

            match op.kind {
                OperationKind::CreateDir => {
                    info!("Creating dir: {}", working_file.display());
                    fs::create_dir_all(working_file).unwrap();
                }
                OperationKind::RemoveDir => {
                    if working_file.read_dir().unwrap().next().is_none() {
                        info!("Removing dir: {}", working_file.display());
                        fs::remove_dir(working_file).unwrap();
                    }
                }
                OperationKind::CreateFile(source) => {
                    let mod_file = self.slotmap[source].dir.join(path);
                    info!("Creating file with hard link: {} -> {} ({})", mod_file.display(), working_file.display(), self.slotmap[source].metadata.name);
                    // check if file exists
                    if working_file.exists() {
                        if !back_file.exists() {
                            trace!(" - Creating backup: {}", back_file.display());
                            fs::create_dir_all(back_file.parent().unwrap()).unwrap();
                            fs::hard_link(&working_file, back_file).unwrap();
                        }
                        trace!(" - Removing file: {}", working_file.display());
                        fs::remove_file(&working_file).unwrap();
                    }
                    fs::create_dir_all(working_file.parent().unwrap()).unwrap();
                    trace!(" - Creating hard link");
                    fs::hard_link(mod_file, working_file).unwrap();
                }
                OperationKind::RemoveFile => {
                    info!("Removing file: {}", working_file.display());
                    fs::remove_file(&working_file).unwrap();
                    if back_file.exists() {
                        trace!(" - Restoring backup with hard link: {} -> {}", back_file.display(), working_file.display());
                        fs::hard_link(&back_file, &working_file).unwrap();
                        fs::remove_file(back_file).unwrap();
                    }
                }
                OperationKind::ChangeSource(new_source) => {
                    info!("Changing source: {} ({})", working_file.display(), self.slotmap[new_source].metadata.name);
                    let mod_file = self.slotmap[new_source].dir.join(path);
                    if working_file.exists() {
                        trace!(" - Removing file: {}", working_file.display());
                        fs::remove_file(&working_file).unwrap();
                    }
                    fs::create_dir_all(working_file.parent().unwrap()).unwrap();
                    trace!(" - Creating hard link: {} -> {}", working_file.display(), mod_file.display());
                    fs::hard_link(mod_file, working_file).unwrap();
                }
            }
        }
    }

    pub fn print_tree(&self) {
        self.current_active_tree.print(0);
    }
}
