use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use pumpkin_plugin_api::command::CommandSender;
use pumpkin_plugin_api::{Player, Result};
use serde::Deserialize;
use toml::{Table, Value};

use crate::config;

const DEFAULT_CONFIG: &str = include_str!("../config/language.toml");
const FALLBACK_LOCALE: &str = "en_us";
const BUNDLED: &[(&str, &str)] = &[
    (FALLBACK_LOCALE, include_str!("../config/lang/en_us.toml")),
    ("zh_cn", include_str!("../config/lang/zh_cn.toml")),
];

static LANGUAGES: Mutex<Option<Languages>> = Mutex::new(None);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageConfig {
    default: String,
}

struct Languages {
    by_locale: HashMap<String, Arc<Messages>>,
    default: Arc<Messages>,
    fallback: Arc<Messages>,
}

#[derive(Debug, Default)]
pub(crate) struct Messages(HashMap<String, String>);

impl Messages {
    /// Returns the message for `key`, or the key itself if it is missing.
    pub(crate) fn text<'a>(&'a self, key: &'a str) -> &'a str {
        self.0.get(key).map_or_else(
            || {
                tracing::error!(key, "Missing translation");
                key
            },
            String::as_str,
        )
    }

    /// Replaces each `{name}` placeholder in the message with the matching argument.
    pub(crate) fn format(&self, key: &str, args: &[(&str, &dyn Display)]) -> String {
        let mut text = String::new();
        let mut rest = self.text(key);
        while let Some(start) = rest.find('{') {
            text.push_str(&rest[..start]);
            rest = &rest[start + 1..];
            let argument = rest.find('}').and_then(|end| {
                let name = &rest[..end];
                let (_, value) = args.iter().find(|(arg, _)| *arg == name)?;
                Some((end, value))
            });
            match argument {
                Some((end, value)) => {
                    text.push_str(&value.to_string());
                    rest = &rest[end + 1..];
                }
                None => text.push('{'),
            }
        }
        text.push_str(rest);
        text
    }
}

#[cfg(test)]
impl Messages {
    pub(crate) fn bundled(locale: &str) -> Self {
        let (_, contents) = BUNDLED.iter().find(|(name, _)| *name == locale).unwrap();
        Self(flatten(&toml::from_str(contents).unwrap()).unwrap())
    }
}

/// Loads `language.toml` and every `lang/*.toml` file, writing bundled defaults when missing.
pub(crate) fn load(directory: &Path) -> Result<()> {
    let languages = Languages::load(directory)?;
    *LANGUAGES.lock().expect("languages lock") = Some(languages);
    Ok(())
}

/// Returns messages in the player's client language, or English if no file matches it.
pub(crate) fn for_player(player: &Player) -> Arc<Messages> {
    let locale = player.get_locale();
    select(Some(&locale))
}

/// Returns messages for a player sender, or in the server default language otherwise.
pub(crate) fn for_sender(sender: &CommandSender) -> Arc<Messages> {
    let locale = sender.as_player().map(|player| player.get_locale());
    select(locale.as_deref())
}

pub(crate) fn server_default() -> Arc<Messages> {
    select(None)
}

fn select(locale: Option<&str>) -> Arc<Messages> {
    LANGUAGES
        .lock()
        .expect("languages lock")
        .as_ref()
        .map(|languages| Arc::clone(languages.select(locale)))
        .unwrap_or_default()
}

impl Languages {
    fn load(directory: &Path) -> Result<Self> {
        let config_path = directory.join("language.toml");
        let config: LanguageConfig = config::load(&config_path, DEFAULT_CONFIG)?;
        let lang_directory = directory.join("lang");
        fs::create_dir_all(&lang_directory)
            .map_err(|error| format!("Cannot create {}: {error}", lang_directory.display()))?;

        let mut bundled = HashMap::new();
        for &(locale, contents) in BUNDLED {
            let table = toml::from_str(contents)
                .map_err(|error| format!("Cannot parse bundled {locale} messages: {error}"))?;
            bundled.insert(locale, flatten(&table)?);
        }
        let reference = bundled
            .get(FALLBACK_LOCALE)
            .ok_or("Bundled English messages are missing")?;

        let mut files = Vec::new();
        for &(locale, contents) in BUNDLED {
            let path = lang_directory.join(format!("{locale}.toml"));
            let table: Table = config::load(&path, contents)?;
            files.push((locale.to_owned(), path, table));
        }
        let entries = fs::read_dir(&lang_directory)
            .map_err(|error| format!("Cannot read {}: {error}", lang_directory.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("Cannot read {}: {error}", lang_directory.display()))?
                .path();
            let locale = match path.file_stem().and_then(|stem| stem.to_str()) {
                Some(stem) if path.extension().is_some_and(|ext| ext == "toml") => normalize(stem),
                _ => continue,
            };
            if bundled.contains_key(locale.as_str()) {
                continue;
            }
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
            let table = toml::from_str(&contents)
                .map_err(|error| format!("Cannot parse {}: {error}", path.display()))?;
            files.push((locale, path, table));
        }

        let mut by_locale = HashMap::new();
        for (locale, path, table) in files {
            let base = bundled.get(locale.as_str()).unwrap_or(reference);
            let messages = merge(base, reference, &table)
                .map_err(|error| format!("Cannot load {}: {error}", path.display()))?;
            by_locale.insert(locale, Arc::new(messages));
        }

        let default = by_locale
            .get(&normalize(&config.default))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "{}: default language {:?} has no file in {}",
                    config_path.display(),
                    config.default,
                    lang_directory.display()
                )
            })?;
        let fallback = by_locale
            .get(FALLBACK_LOCALE)
            .cloned()
            .ok_or("English messages are missing")?;
        Ok(Self {
            by_locale,
            default,
            fallback,
        })
    }

    fn select(&self, locale: Option<&str>) -> &Arc<Messages> {
        match locale.filter(|locale| !locale.is_empty()) {
            Some(locale) => self
                .by_locale
                .get(&normalize(locale))
                .unwrap_or(&self.fallback),
            None => &self.default,
        }
    }
}

/// Overlays a language file on `base`, rejecting keys and placeholders that English does not define.
fn merge(
    base: &HashMap<String, String>,
    reference: &HashMap<String, String>,
    table: &Table,
) -> Result<Messages> {
    let mut messages = base.clone();
    for (key, text) in flatten(table)? {
        let expected = reference
            .get(&key)
            .ok_or_else(|| format!("unknown key `{key}`"))?;
        let allowed = placeholders(expected);
        if let Some(name) = placeholders(&text)
            .into_iter()
            .find(|name| !allowed.contains(name))
        {
            return Err(format!("`{key}` uses unknown placeholder {{{name}}}"));
        }
        messages.insert(key, text);
    }
    Ok(Messages(messages))
}

/// Converts nested TOML tables into dotted message keys.
fn flatten(table: &Table) -> Result<HashMap<String, String>> {
    fn visit(prefix: &str, table: &Table, messages: &mut HashMap<String, String>) -> Result<()> {
        for (key, value) in table {
            let key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value {
                Value::String(text) => {
                    messages.insert(key, text.clone());
                }
                Value::Table(table) => visit(&key, table, messages)?,
                _ => return Err(format!("`{key}` must be a string")),
            }
        }
        Ok(())
    }

    let mut messages = HashMap::new();
    visit("", table, &mut messages)?;
    Ok(messages)
}

fn placeholders(text: &str) -> HashSet<&str> {
    text.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name))
        .collect()
}

/// Matches client locales such as `en_US` or `en-us` to file names such as `en_us`.
fn normalize(locale: &str) -> String {
    locale.to_ascii_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::abilities::ability_manager::AbilityId;
    use crate::skills::skill_manager::SkillType;

    #[test]
    fn bundled_languages_define_the_same_keys_and_placeholders() {
        let english = Messages::bundled(FALLBACK_LOCALE).0;
        let chinese = Messages::bundled("zh_cn").0;
        let mut keys = english.keys().collect::<Vec<_>>();
        keys.sort();
        let mut chinese_keys = chinese.keys().collect::<Vec<_>>();
        chinese_keys.sort();
        assert_eq!(keys, chinese_keys);
        for key in keys {
            assert_eq!(
                placeholders(&english[key]),
                placeholders(&chinese[key]),
                "{key}"
            );
        }
        for key in [
            SkillType::Mining.name_key(),
            SkillType::Axes.name_key(),
            AbilityId::SuperBreaker.name_key(),
            AbilityId::Berserk.name_key(),
        ] {
            assert!(english.contains_key(key), "{key}");
        }
    }

    #[test]
    fn formats_each_placeholder_once() {
        let messages = Messages(HashMap::from([(
            "greeting".to_owned(),
            "{name} has {count} {unknown}".to_owned(),
        )]));
        let text = messages.format("greeting", &[("name", &"{count}"), ("count", &3)]);
        assert_eq!(text, "{count} has 3 {unknown}");
        assert_eq!(messages.format("missing", &[]), "missing");
    }

    #[test]
    fn loads_language_files_and_selects_by_locale() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "rummo-i18n-{}-{}",
            std::process::id(),
            timestamp.as_nanos()
        ));
        let lang = directory.join("lang");
        fs::create_dir_all(&lang).unwrap();
        fs::write(lang.join("zh_cn.toml"), "[skill]\nmining = \"采矿\"\n").unwrap();
        fs::write(lang.join("ja_jp.toml"), "[skill]\nmining = \"採掘\"\n").unwrap();

        let languages = Languages::load(&directory).unwrap();
        assert_eq!(
            fs::read_to_string(lang.join("en_us.toml")).unwrap(),
            BUNDLED[0].1
        );
        let text = |locale| {
            languages
                .select(locale)
                .text(SkillType::Mining.name_key())
                .to_owned()
        };
        assert_eq!(text(Some("zh_CN")), "采矿");
        assert_eq!(text(Some("ja-jp")), "採掘");
        assert_eq!(text(Some("fr_fr")), "Mining");
        assert_eq!(text(Some("")), "采矿");
        assert_eq!(text(None), "采矿");
        assert_eq!(languages.select(Some("zh_cn")).text("skill.axes"), "斧技");
        assert_eq!(languages.select(Some("ja_jp")).text("skill.axes"), "Axes");

        for (contents, error) in [
            ("[skill]\nfishing = \"x\"\n", "unknown key `skill.fishing`"),
            (
                "[skill]\nmining = \"{level}\"\n",
                "unknown placeholder {level}",
            ),
            ("[skill]\nmining = 1\n", "`skill.mining` must be a string"),
        ] {
            fs::write(lang.join("ja_jp.toml"), contents).unwrap();
            let message = Languages::load(&directory).err().unwrap();
            assert!(message.contains(error), "{message}");
        }
        fs::remove_file(lang.join("ja_jp.toml")).unwrap();
        fs::write(directory.join("language.toml"), "default = \"fr_fr\"\n").unwrap();
        let message = Languages::load(&directory).err().unwrap();
        assert!(message.contains("\"fr_fr\" has no file"), "{message}");

        fs::remove_dir_all(directory).unwrap();
    }
}
