use std::collections::HashMap;
use std::process::Command;
use std::{thread, time};

#[derive(Clone, Debug)]
pub struct Disc {
    pub name: String,
    pub r#type: Option<DiscType>,
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum DiscType {
    BluRay,
    Data,
    DVD,
    Music,
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
        use heck::ShoutySnakeCase;

        self.title().to_shouty_snake_case()
    }

    pub fn title(&self) -> String {
        use heck::TitleCase;

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
    let sys_name = device.split("/").nth(2).unwrap();

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

    if let Some(_) = properties.get("ID_CDROM_MEDIA_BD") {
        return Some(DiscType::BluRay);
    }

    if let Some(_) = properties.get("ID_CDROM_MEDIA_DVD") {
        return Some(DiscType::DVD);
    }

    if let Some(_) = properties.get("ID_CDROM_MEDIA_TRACK_COUNT_AUDIO") {
        return Some(DiscType::Music);
    }

    None
}

pub fn eject(disc: &Disc) {
    Command::new("eject")
        .arg(&disc.name)
        .output()
        .expect("failed to execute process");

    thread::sleep(time::Duration::from_secs(2));
}
