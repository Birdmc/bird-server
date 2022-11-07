use std::fmt::{Display, Formatter};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
#[serde(try_from = "&str", into = "String")]
pub enum Color {
    Black,
    DarkBlue,
    DarkGreen,
    DarkCyan,
    DarkRed,
    Purple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    BrightGreen,
    Cyan,
    Red,
    Pink,
    Yellow,
    White,
    Custom { r: u8, g: u8, b: u8 },
}

impl Color {
    pub const fn get_color(&self) -> u32 {
        match self {
            Self::Black => 0x000000,
            Self::DarkBlue => 0x0000aa,
            Self::DarkGreen => 0x00aa00,
            Self::DarkCyan => 0x00aaaa,
            Self::DarkRed => 0xaa0000,
            Self::Purple => 0xaa00aa,
            Self::Gold => 0xffaa00,
            Self::Gray => 0xaaaaaa,
            Self::DarkGray => 0x555555,
            Self::Blue => 0x5555ff,
            Self::BrightGreen => 0x55ff55,
            Self::Cyan => 0x55ffff,
            Self::Red => 0xff5555,
            Self::Pink => 0xff55ff,
            Self::Yellow => 0xffff55,
            Self::White => 0xffffff,
            Self::Custom { r, g, b } => (*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)
        }
    }

    pub const fn from_color(color: u32) -> Self {
        match color {
            0x000000 => Self::Black,
            0x0000aa => Self::DarkBlue,
            0x00aa00 => Self::DarkGreen,
            0x00aaaa => Self::DarkCyan,
            0xaa0000 => Self::DarkRed,
            0xaa00aa => Self::Purple,
            0xffaa00 => Self::Gold,
            0xaaaaaa => Self::Gray,
            0x555555 => Self::DarkGray,
            0x5555ff => Self::Blue,
            0x55ff55 => Self::BrightGreen,
            0x55ffff => Self::Cyan,
            0xff5555 => Self::Red,
            0xff55ff => Self::Purple,
            0xffff55 => Self::Yellow,
            0xffffff => Self::White,
            other => Self::Custom {
                r: (other >> 16 & 0xff) as u8,
                g: (other >> 8 & 0xff) as u8,
                b: (other & 0xff) as u8,
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Parsing of color is failed")]
pub struct ColorParseError;

impl TryFrom<&str> for Color {
    type Error = ColorParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.starts_with('#') && value.len() == 7 {
            true => Ok(Self::Custom {
                r: u8::from_str_radix(&value[1..=2], 16).map_err(|_| ColorParseError)?,
                g: u8::from_str_radix(&value[3..=4], 16).map_err(|_| ColorParseError)?,
                b: u8::from_str_radix(&value[5..=6], 16).map_err(|_| ColorParseError)?,
            }),
            false => match value {
                "black" => Ok(Self::Black),
                "dark_blue" => Ok(Self::DarkBlue),
                "dark_green" => Ok(Self::DarkGreen),
                "dark_aqua" => Ok(Self::DarkCyan),
                "dark_red" => Ok(Self::DarkRed),
                "dark_purple" => Ok(Self::Purple),
                "gold" => Ok(Self::Gold),
                "gray" => Ok(Self::Gray),
                "dark_gray" => Ok(Self::DarkGray),
                "blue" => Ok(Self::Blue),
                "green" => Ok(Self::BrightGreen),
                "aqua" => Ok(Self::Cyan),
                "red" => Ok(Self::Red),
                "light_purple" => Ok(Self::Pink),
                "yellow" => Ok(Self::Yellow),
                "white" => Ok(Self::White),
                _ => Err(ColorParseError),
            },
        }
    }
}

impl TryFrom<String> for Color {
    type Error = ColorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Color::try_from(value.as_str())
    }
}

impl From<Color> for String {
    fn from(value: Color) -> Self {
        value.to_string()
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Black => write!(f, "black"),
            Self::DarkBlue => write!(f, "dark_blue"),
            Self::DarkGreen => write!(f, "dark_green"),
            Self::DarkCyan => write!(f, "dark_aqua"),
            Self::DarkRed => write!(f, "dark_red"),
            Self::Purple => write!(f, "dark_purple"),
            Self::Gold => write!(f, "gold"),
            Self::Gray => write!(f, "gray"),
            Self::DarkGray => write!(f, "dark_gray"),
            Self::Blue => write!(f, "blue"),
            Self::BrightGreen => write!(f, "green"),
            Self::Cyan => write!(f, "aqua"),
            Self::Red => write!(f, "red"),
            Self::Pink => write!(f, "light_purple"),
            Self::Yellow => write!(f, "yellow"),
            Self::White => write!(f, "white"),
            Self::Custom { r, g, b } => write!(f, "#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::color::Color;

    #[test]
    fn color_serde() {
        assert_eq!(serde_json::to_string(&Color::Black).unwrap(), r#""black""#);
        assert_eq!(serde_json::to_string(&Color::Cyan).unwrap(), r#""aqua""#);
        assert_eq!(serde_json::to_string(&Color::Custom { r: 255, g: 255, b: 255 }).unwrap(), "\"#ffffff\"");
        assert_eq!(serde_json::to_string(&Color::Custom { r: 16, g: 32, b: 255 }).unwrap(), "\"#1020ff\"");
    }
}
