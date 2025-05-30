use std::{fs, path::PathBuf, slice::GetDisjointMutError};

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Config {
    hp: i32,
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
