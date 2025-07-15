use std::{collections::HashMap, time::Duration};

use heck::{ToShoutySnekCase, ToTitleCase};
use serde::{Deserialize, Serialize};
use tokio::{process::Command, time::sleep};

#[derive(Clone, Debug)]
pub struct Disc {
    pub name: String,
    pub r#type: Option<DiscType>,
    pub properties: HashMap<String, String>,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum DiscType {
    BluRay,
    Data,
    Dvd,
    Music,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct DiscMetadata {
    pub disc_type: DiscType,
}

impl Disc {
    pub fn new(device: &str) -> Disc {
        let properties = get_device_proprties(device);

        Disc {
            name: device.to_owned(),
            r#type: get_device_type(&properties),
            properties,
        }
    }

    pub fn path_friendly_title(&self) -> String {
        self.title().TO_SHOUTY_SNEK_CASE()
    }

    pub fn title(&self) -> String {
        if let Some(val) = self.properties.get("ID_FS_LABEL") {
            if val != "iso9660" {
                return val.to_title_case();
            }
        }

        if let Some(val) = self.properties.get("ID_FS_UUID") {
            return val.to_owned();
        }

        unimplemented!()
    }
}

fn get_device_proprties(device: &str) -> HashMap<String, String> {
    let sys_name = device.split('/').nth(2).unwrap();

    let mut enumerator = udev::Enumerator::new().unwrap();

    enumerator.match_subsystem("block").unwrap();

    enumerator.match_sysname(sys_name).unwrap();

    let mut result = HashMap::new();

    for device in enumerator.scan_devices().unwrap() {
        for p in device.properties() {
            result.insert(
                p.name().to_str().unwrap().to_owned(),
                p.value().to_str().unwrap().to_owned(),
            );
        }
    }

    result
}

fn get_device_type(properties: &HashMap<String, String>) -> Option<DiscType> {
    if let Some(val) = properties.get("ID_FS_LABEL") {
        if val == "iso9660" {
            return Some(DiscType::Data);
        }
    }

    if properties.get("ID_CDROM_MEDIA_BD").is_some() {
        return Some(DiscType::BluRay);
    }

    if properties.get("ID_CDROM_MEDIA_DVD").is_some() {
        return Some(DiscType::Dvd);
    }

    if properties.get("ID_CDROM_MEDIA_TRACK_COUNT_AUDIO").is_some() {
        return Some(DiscType::Music);
    }

    None
}

pub async fn eject(disc: &Disc) {
    Command::new("eject")
        .arg(&disc.name)
        .output()
        .await
        .expect("failed to execute process");

    sleep(Duration::from_secs(2)).await;
}
