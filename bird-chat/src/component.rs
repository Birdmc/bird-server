use std::borrow::Cow;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::color::Color;
use crate::identifier::Identifier;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Component<'a> {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub font: Option<Identifier<'a>>,
    pub color: Option<Color>,
    pub insertion: Option<Cow<'a, str>>,
    pub click_event: Option<ClickEvent<'a>>,
    pub extra: Cow<'a, [Component<'a>]>,
    pub hover_event: Option<HoverEvent<'a>>,
    #[serde(flatten)]
    pub ty: Option<ComponentType<'a>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "action", content = "value")]
pub enum ClickEvent<'a> {
    OpenUrl(Cow<'a, str>),
    RunCommand(Cow<'a, str>),
    SuggestCommand(Cow<'a, str>),
    ChangePage(i32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "action", content = "value")]
pub enum HoverEvent<'a> {
    ShowText(either::Either<Box<Component<'a>>, Cow<'a, str>>),
    ShowItem(Cow<'a, str>),
    ShowEntity(Cow<'a, str>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum ComponentType<'a> {
    Text {
        text: Cow<'a, str>,
    },
    Translation {
        with: Cow<'a, [Component<'a>]>,
        key: Cow<'a, str>,
    },
    KeyBind {
        #[serde(rename = "keybind")]
        key_bind: Cow<'a, str>,
    },
    Selector {
        selector: Cow<'a, str>,
    },
    Score {
        score: Score<'a>,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Score<'a> {
    name: Cow<'a, str>, // possible uuid but actually string in json
    objective: Cow<'a, str>,
    value: Cow<'a, str>,
}