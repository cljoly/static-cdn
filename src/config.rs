/* Copyright © 2025 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // TODO Pull that from the API, would be more ergonomic. Then replace witrh the site name
    // (i.e. cj.rs)
    pub site_uuid: String,
    pub api_token_cmd: String,
}

const PATH: &'static str = concat!(env!("CARGO_PKG_NAME"), ".toml");
static DEFAULT_CONTENT: &'static str = include_str!("default-config.toml");

pub fn load() -> Result<Config> {
    let path = Path::new(PATH);

    let mut s = String::new();
    let content = if path.exists() {
        let mut file = File::open(path)?;
        file.read_to_string(&mut s)?;
        &s
    } else {
        let mut file = File::create(PATH)?;
        file.write_all(DEFAULT_CONTENT.as_bytes())?;
        DEFAULT_CONTENT
    };
    Ok(basic_toml::from_str(&content)?)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn default_config() -> Result<()> {
        let _: Config = basic_toml::from_str(&DEFAULT_CONTENT)?;
        Ok(())
    }
}
