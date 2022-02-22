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

use core::convert::From;
use core::default::Default;
use std::fmt::{Debug, Formatter};

pub struct Area {
    width: usize,
    height: usize,
}

impl Debug for Area {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut text = self.width.to_string();
        text.push('×');
        text.push_str(&self.height.to_string());

        f.debug_tuple("Area").field(&text).finish()
    }
}

impl Area {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }
}

impl From<(u16, u16)> for Area {
    fn from(size: (u16, u16)) -> Self {
        Self {
            width: size.0.try_into().expect("u16 to usize"),
            height: size.1.try_into().expect("u16 to usize"),
        }
    }
}

impl Default for Area {
    fn default() -> Self {
        Self {
            width: 80,
            height: 25,
        }
    }
}
