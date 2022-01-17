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

use crossterm::style::{Color, ContentStyle};
lazy_static::lazy_static! {
    pub static ref DEFAULT_STYLE: ContentStyle = ContentStyle::new();
    pub static ref ID_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkMagenta),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref DATE_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkBlue),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref MOD_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkYellow),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref NAME_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkGreen),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref REF_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkCyan),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
}
