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
        Area { width, height }
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
        Area {
            width: size.0 as usize,
            height: size.1 as usize,
        }
    }
}

impl Default for Area {
    fn default() -> Self {
        Area {
            width: 80,
            height: 25,
        }
    }
}
