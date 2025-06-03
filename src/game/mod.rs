use std::{
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub mod parser;

pub const FILE_SIGNATURE: &[u8; 8] = b"\x00rlsh0.1";

pub const LOCKED_DOOR_ICON: &'static str = "󱂯";
pub const UNLOCKED_DOOR_ICON: &'static str = "󰠛";
pub const PERSON_ICON: &'static str = "";

#[derive(Deserialize, Serialize)]
struct Config {
    hp: i32,
}

/// An action that
enum Action {
    Attack,
}

#[derive(Deserialize, Serialize)]
pub struct Entity {
    pub components: Vec<Component>,
}

impl TryFrom<&[u8]> for Entity {
    type Error = rmp_serde::decode::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        rmp_serde::from_slice(buffer)
    }
}

impl From<Entity> for Vec<u8> {
    fn from(e: Entity) -> Self {
        rmp_serde::to_vec(&e).unwrap()
    }
}

impl Entity {
    fn act(&self, action: Action) {}
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Component {
    Enemy,
    TakesDamage(i16),
    Retaliates(i16),
    HasInventory(Vec<String>),
}

/// "Spawns" an entity in the specified path (relative to the current working directory),
/// Its name will be its filename.
pub fn spawn(e: Entity, path: impl AsRef<Path>) {
    let mut abs_path = env::current_dir().unwrap();
    abs_path.push(path);

    let mut contents = Vec::new();
    contents.extend_from_slice(FILE_SIGNATURE);
    contents.extend(Vec::from(e).into_iter());
    fs::write(abs_path, contents).unwrap();
}

pub fn get_entity(path: impl AsRef<Path>) -> Result<Entity, Box<dyn std::error::Error>> {
    let mut entity_buffer = Vec::new();
    {
        let mut f = File::open(path)?;
        let mut file_sig_check_buffer = [0; FILE_SIGNATURE.len()];
        f.read_exact(&mut file_sig_check_buffer)?;
        if file_sig_check_buffer != *FILE_SIGNATURE {
            return Err(Box::from("lol"));
        }

        f.read_to_end(&mut entity_buffer)?;
        // the file is closed at the end of this scope
    }

    match Entity::try_from(&entity_buffer[..]) {
        Ok(e) => Ok(e),
        Err(_) => Err(Box::from("yikes")),
    }
}

pub fn attack(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut abs_path = env::current_dir().unwrap();
    abs_path.push(&path);

    let e = get_entity(&abs_path)?;

    for c in &e.components {
        match c {
            Component::TakesDamage(hp) => {
                fs::remove_file(&abs_path)?;
                println!("the dude has {} hp.", hp);
                let damage = rand::random_range(1..4);
                let new_hp = hp - damage;
                if new_hp <= 0 {
                    println!("you punched him so hard he died. yikes.");
                    return Ok(());
                }
                println!(
                    "you punched him with some amount of force, knocking out about {} teeth.",
                    damage
                );
                println!("the poor sod only has {} left.", new_hp);
                spawn(
                    Entity {
                        components: e
                            .components
                            .clone()
                            .into_iter()
                            .filter(|c| {
                                std::mem::discriminant(c)
                                    != std::mem::discriminant(&Component::TakesDamage(0))
                            })
                            .chain([Component::TakesDamage(new_hp)].into_iter())
                            .collect(),
                    },
                    &abs_path,
                );
            }
            _ => (),
        }
    }

    Ok(())
}

/// Returns the path to the file we use for all rlsh data.
/// This includes game data like the current HP and configuration data like
/// any name or preference changes.
fn get_data_path() -> PathBuf {
    let mut path = dirs::data_local_dir().expect("Could not find the data path :(");
    path.push("/rlsh");
    path.push("save.cfg");
    path
}

pub fn check_setup() {
    let f = fs::read_to_string(get_data_path());
}
