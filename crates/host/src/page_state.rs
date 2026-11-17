//! What the cabin page keeps between runs (its settings, recent destinations, Safari's tabs, the players' kept
//! state), kept here rather than in the page's own browser storage, so every page WestPlay shows (the truck's, the
//! iPad's) has the same. The page names each part by its storage key (`westplay.recents`) and hands its value as
//! text; this only stores them.
//!
//! Saved as `page_state.json` in WestPlay's data folder. No file means nothing was ever kept: the page then hands
//! over what it kept itself, once ([`PageState::import`]), so a player updating loses nothing. An unreadable file is
//! set aside (`page_state.json.bad`) and counts as none, so the page's own copy is taken again.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Every key the page keeps starts with this.
pub const KEY_PREFIX: &str = "westplay.";
/// The longest key, the largest value and the most kept in all (the iPad is a client on the network).
pub const KEY_MOST: usize = 64;
pub const VALUE_MOST: usize = 1 << 20;
pub const TOTAL_MOST: usize = 8 << 20;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    values: BTreeMap<String, String>,
}

pub struct PageState {
    path: PathBuf,
    /// `None`: nothing ever kept (the page's own copy is taken once).
    values: Option<BTreeMap<String, String>>,
}

impl PageState {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("page_state.json");
        let values = match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<File>(&json) {
                Ok(file) => Some(file.values),
                Err(e) => {
                    let aside = path.with_extension("json.bad");
                    log!("{} is not valid ({e}); moved to {}", path.display(), aside.display());
                    let _ = std::fs::rename(&path, aside);
                    None
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => {
                log!("cannot read {}: {e}", path.display());
                None
            }
        };
        Self { path, values }
    }

    /// `{"type":"pageState","values":{...}}`, or `"values":null` when nothing was ever kept.
    pub fn all_message(&self) -> String {
        serde_json::json!({ "type": "pageState", "values": self.values }).to_string()
    }

    /// Keeps `value` for `key` (`None` forgets it). Returns the message telling the pages, or `None` when nothing
    /// changed or the key or value isn't one kept.
    pub fn set(&mut self, key: &str, value: Option<&str>) -> Option<String> {
        if !valid_key(key) || value.is_some_and(|value| value.len() > VALUE_MOST) {
            log!("page state: {key:.80} not kept");
            return None;
        }
        let values = self.values.get_or_insert_with(BTreeMap::new);
        if values.get(key).map(String::as_str) == value {
            return None;
        }
        match value {
            Some(value) => {
                let total: usize = values.iter().filter(|(k, _)| k.as_str() != key).map(|(k, v)| k.len() + v.len()).sum();
                if total + key.len() + value.len() > TOTAL_MOST {
                    log!("page state: {key} not kept (too much kept in all)");
                    return None;
                }
                values.insert(key.to_owned(), value.to_owned());
            }
            None => {
                values.remove(key);
            }
        }
        self.save();
        Some(serde_json::json!({ "type": "pageStateChanged", "key": key, "value": value }).to_string())
    }

    /// The page's own copy, taken only when nothing was ever kept. Returns the message telling the pages.
    pub fn import(&mut self, values: BTreeMap<String, String>) -> Option<String> {
        if self.values.is_some() {
            return None;
        }
        let mut kept = BTreeMap::new();
        let mut total = 0;
        for (key, value) in values {
            if valid_key(&key) && value.len() <= VALUE_MOST && total + key.len() + value.len() <= TOTAL_MOST {
                total += key.len() + value.len();
                kept.insert(key, value);
            }
        }
        self.values = Some(kept);
        self.save();
        Some(self.all_message())
    }

    fn save(&self) {
        let file = File { version: 1, values: self.values.clone().unwrap_or_default() };
        let write = || -> io::Result<()> {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let partial = self.path.with_extension("json.partial");
            std::fs::write(&partial, serde_json::to_string(&file).expect("page state serializes"))?;
            std::fs::rename(partial, &self.path)
        };
        if let Err(e) = write() {
            log!("cannot write {}: {e}", self.path.display());
        }
    }
}

fn valid_key(key: &str) -> bool {
    key.starts_with(KEY_PREFIX) && key.len() <= KEY_MOST
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("westplay-page-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nothing_kept_says_null_then_imports_once() {
        let dir = dir("import");
        let mut state = PageState::load(&dir);
        assert_eq!(state.all_message(), r#"{"type":"pageState","values":null}"#);
        let values = BTreeMap::from([("westplay.recents".to_owned(), "R".to_owned()), ("other".to_owned(), "x".to_owned())]);
        assert_eq!(state.import(values).unwrap(), r#"{"type":"pageState","values":{"westplay.recents":"R"}}"#);
        assert_eq!(state.import(BTreeMap::new()), None);
        assert_eq!(PageState::load(&dir).all_message(), r#"{"type":"pageState","values":{"westplay.recents":"R"}}"#);
    }

    #[test]
    fn tells_only_changes_and_refuses_foreign_or_oversized() {
        let dir = dir("set");
        let mut state = PageState::load(&dir);
        assert!(state.set("westplay.recents", Some("A")).is_some());
        assert_eq!(state.set("westplay.recents", Some("A")), None);
        assert_eq!(state.set("other", Some("A")), None);
        assert_eq!(state.set(&format!("westplay.{}", "k".repeat(60)), Some("A")), None);
        assert_eq!(state.set("westplay.big", Some(&"x".repeat(VALUE_MOST + 1))), None);
        assert_eq!(
            state.set("westplay.recents", None).unwrap(),
            r#"{"key":"westplay.recents","type":"pageStateChanged","value":null}"#
        );
        assert_eq!(PageState::load(&dir).all_message(), r#"{"type":"pageState","values":{}}"#);
    }

    #[test]
    fn unreadable_file_is_set_aside_and_counts_as_never_kept() {
        let dir = dir("bad");
        std::fs::write(dir.join("page_state.json"), "{not json").unwrap();
        let mut state = PageState::load(&dir);
        assert!(dir.join("page_state.json.bad").exists());
        assert!(state.import(BTreeMap::from([("westplay.a".to_owned(), "1".to_owned())])).is_some());
    }
}
