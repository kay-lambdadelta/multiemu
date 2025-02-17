use multiemu_rom::id::RomId;
use multiemu_rom::system::GameSystem;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Datafile {
    pub header: Header,
    #[serde(alias = "game")]
    pub machine: Vec<Machine>,
}

#[allow(dead_code)]
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Header {
    #[serde_as(as = "DisplayFromStr")]
    pub name: GameSystem,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Machine {
    #[serde(rename = "@name")]
    pub name: String,
    pub description: String,
    pub rom: Vec<Rom>,
}

#[allow(dead_code)]
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Rom {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "@sha1")]
    pub id: RomId,
    pub status: Option<String>,
}
