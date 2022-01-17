// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use configparser::ini::Ini;
use lazy_static::lazy_static;
use xdg::BaseDirectories;

lazy_static! {
    static ref CONFIG: Ini = config();
}

fn config() -> Ini {
    let xdg_dirs = BaseDirectories::with_prefix("glv").expect("Expected BaseDirectories");
    let mut result = Ini::new();
    match xdg_dirs.find_config_file("config") {
        None => {}
        Some(config_path) => {
            let path = config_path
                .to_str()
                .expect("A path convertible to an UTF-8 string");
            result.load(path).expect("Loaded INI file");
        }
    }
    result
}

pub fn author_name_width() -> usize {
    match CONFIG.getuint("history", "author_name_width") {
        Ok(o) => match o {
            None => 10,
            #[allow(clippy::cast_possible_truncation)]
            Some(v) => v as usize,
        },
        Err(e) => panic!("Error while parsing history.author_name_width: {}", e),
    }
}

pub fn author_rel_date_width() -> usize {
    match CONFIG.getuint("history", "author_rel_date_width") {
        Ok(o) => match o {
            None => 0,
            #[allow(clippy::cast_possible_truncation)]
            Some(v) => v as usize,
        },
        Err(e) => panic!("Error while parsing history.author_rel_name_width: {}", e),
    }
}

pub fn modules_width() -> usize {
    match CONFIG.getuint("history", "modules_width") {
        Ok(o) => match o {
            None => 35,
            #[allow(clippy::cast_possible_truncation)]
            Some(v) => v as usize,
        },
        Err(e) => panic!("Error while parsing history.modules_width: {}", e),
    }
}
