use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::BTreeMap;
use versions::SemVer;

mod component_name;
pub use component_name::*;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub component_version: SemVer,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SaveMetadata {
    pub components: BTreeMap<ComponentName, ComponentInfo>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SnapshotMetadata {
    pub components: BTreeMap<ComponentName, ComponentInfo>,
}
