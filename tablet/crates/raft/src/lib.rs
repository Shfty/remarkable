//! Parser for draft application files
use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::PathBuf, error::Error,
};

pub const DRAFT_PATH: &'static str = "/opt/etc/draft";
pub const ICONS_DIR: &'static str = "icons";

#[derive(Debug, Default, Clone)]
pub struct Draft {
    pub name: String,
    pub desc: String,
    pub call: PathBuf,
    pub which: Option<String>,
    pub term: Option<String>,
    pub icon: Option<String>,
}

impl Draft {
    pub fn new(input: &str) -> Result<Self, &'static str> {
        let mut draft = Draft::default();

        for line in input
            .lines()
            .filter(|line| !line.starts_with("#") && !line.is_empty())
        {
            let (key, value) = line.split_once("=").unwrap();
            match key {
                "name" => draft.name = value.to_string(),
                "desc" => draft.desc = value.to_string(),
                "call" => draft.call = value.into(),
                "which" => draft.which = Some(value.to_string()),
                "term" => draft.term = Some(value.to_string()),
                "imgFile" => {
                    draft.icon =
                        Some(DRAFT_PATH.to_owned() + "/" + ICONS_DIR + "/" + value + ".png");
                }
                _ => (),
            }
        }

        if draft.name.is_empty() {
            return Err("Draft has no name");
        }

        if draft.desc.is_empty() {
            return Err("Draft has no description");
        }

        if !draft.call.exists() {
            return Err("Draft launch target does not exist");
        }

        Ok(draft)
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        self.call.file_name()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Drafts(Vec<Draft>);

impl Deref for Drafts {
    type Target = Vec<Draft>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Drafts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drafts {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Drafts({
            let draft_paths = std::fs::read_dir("/opt/etc/draft")?
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension() == Some(OsStr::new("draft")));

            let mut drafts = vec![];
            for path in draft_paths {
                let file = std::fs::read_to_string(path)?;
                drafts.push(Draft::new(&file)?);
            }

            drafts.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

            drafts
        }))
    }

    pub fn take(self) -> Vec<Draft> {
        self.0
    }
}
