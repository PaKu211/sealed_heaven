//! Runtime language resources for living-world presentation (C3.6 phase A).
//! The simulation crate never sees a catalog, locale, or translated value.
use crate::locale::{Locale, parse_dictionary};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const EN: &str = include_str!("../locales/living_en.json");
const ID: &str = include_str!("../locales/living_id.json");
const MAX_PACK_BYTES: u64 = 256 * 1024;
const MAX_MESSAGE_BYTES: usize = 8192;

#[derive(Clone, Debug)]
enum Part {
    Literal(String),
    Parameter(String),
}

#[derive(Clone, Debug)]
struct Message {
    parts: Vec<Part>,
    parameters: BTreeSet<String>,
    plain: Option<String>,
}

impl Message {
    fn parse(value: &str) -> Result<Self, String> {
        if value.len() > MAX_MESSAGE_BYTES {
            return Err("message exceeds byte limit".into());
        }
        if value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        {
            return Err("message contains forbidden control characters".into());
        }
        let mut chars = value.chars().peekable();
        let mut parts = Vec::new();
        let mut parameters = BTreeSet::new();
        let mut literal = String::new();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '{' => {
                    if !literal.is_empty() {
                        parts.push(Part::Literal(std::mem::take(&mut literal)));
                    }
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' => {
                                name.push(c)
                            }
                            _ => return Err("invalid or unclosed parameter".into()),
                        }
                    }
                    if !name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
                        return Err("parameter must start with a-z or underscore".into());
                    }
                    parameters.insert(name.clone());
                    parts.push(Part::Parameter(name));
                }
                '}' => return Err("unescaped closing brace".into()),
                _ => literal.push(c),
            }
        }
        if !literal.is_empty() {
            parts.push(Part::Literal(literal));
        }
        let plain = parameters.is_empty().then(|| {
            parts
                .iter()
                .map(|part| match part {
                    Part::Literal(text) => text.as_str(),
                    Part::Parameter(_) => unreachable!(),
                })
                .collect()
        });
        Ok(Self {
            parts,
            parameters,
            plain,
        })
    }

    fn render(&self, args: &[(&str, &str)]) -> Result<String, String> {
        let mut values = BTreeMap::new();
        for &(name, value) in args {
            if values.insert(name, value).is_some() {
                return Err(format!("duplicate parameter {name}"));
            }
        }
        if !values
            .keys()
            .copied()
            .eq(self.parameters.iter().map(String::as_str))
        {
            return Err("missing or unknown parameters".into());
        }
        let mut result = String::new();
        for part in &self.parts {
            result.push_str(match part {
                Part::Literal(value) => value,
                Part::Parameter(name) => values[name.as_str()],
            });
        }
        Ok(result)
    }
}

/// Validated immutable catalog. English is canonical; selected language falls back
/// to English per missing key. An explicit partial ID file deliberately replaces
/// the embedded Indonesian map, so missing translations are easy to discover.
#[derive(Clone, Debug)]
pub struct LivingCatalog {
    english: BTreeMap<String, Message>,
    indonesian: BTreeMap<String, Message>,
}

fn dictionary(label: &str, json: &str) -> Result<BTreeMap<String, Message>, String> {
    parse_dictionary(label, json)?
        .into_iter()
        .map(|(key, value)| {
            Message::parse(&value)
                .map(|message| (key.clone(), message))
                .map_err(|error| format!("{label}: {key}: {error}"))
        })
        .collect()
}

fn validate_overlay(
    base: &BTreeMap<String, Message>,
    overlay: &BTreeMap<String, Message>,
) -> Result<(), String> {
    for (key, message) in overlay {
        let canonical = base
            .get(key)
            .ok_or_else(|| format!("unknown message ID {key}"))?;
        if message.parameters != canonical.parameters {
            return Err(format!("parameter contract mismatch for {key}"));
        }
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(MAX_PACK_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_PACK_BYTES {
        return Err(format!("{}: catalog exceeds byte limit", path.display()));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| format!("{}: invalid UTF-8: {error}", path.display()))
}

impl LivingCatalog {
    pub fn embedded() -> Result<Self, String> {
        let english = dictionary("living/en", EN)?;
        let indonesian = dictionary("living/id", ID)?;
        validate_overlay(&english, &indonesian)?;
        if !english.keys().eq(indonesian.keys()) {
            return Err("embedded living dictionary key parity mismatch".into());
        }
        Ok(Self {
            english,
            indonesian,
        })
    }

    /// Entire override pair validates before either map is published.
    pub fn with_overrides(english: Option<&str>, indonesian: Option<&str>) -> Result<Self, String> {
        let mut catalog = Self::embedded()?;
        let en = english
            .map(|source| dictionary("external/en", source))
            .transpose()?;
        let id = indonesian
            .map(|source| dictionary("external/id", source))
            .transpose()?;
        if let Some(en) = en {
            validate_overlay(&catalog.english, &en)?;
            catalog.english.extend(en);
        }
        if let Some(id) = id {
            validate_overlay(&catalog.english, &id)?;
            catalog.indonesian = id;
        }
        Ok(catalog)
    }

    pub fn from_directory(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() || !path.is_dir() {
            return Err("locale directory must be an existing absolute path".into());
        }
        let en = read_optional(&path.join("living_en.json"))?;
        let id = read_optional(&path.join("living_id.json"))?;
        if en.is_none() && id.is_none() {
            return Err(
                "locale directory contains neither living_en.json nor living_id.json".into(),
            );
        }
        Self::with_overrides(en.as_deref(), id.as_deref())
    }

    fn message(&self, locale: Locale, key: &str) -> Option<&Message> {
        match locale {
            Locale::English => self.english.get(key),
            Locale::Indonesian => self.indonesian.get(key).or_else(|| self.english.get(key)),
        }
    }

    /// Diagnostic key returned if missing or if a parameterized message was
    /// mistakenly requested through the literal-only interface.
    pub fn text<'a>(&'a self, locale: Locale, key: &'a str) -> &'a str {
        self.message(locale, key)
            .and_then(|message| message.plain.as_deref())
            .unwrap_or(key)
    }

    pub fn format(
        &self,
        locale: Locale,
        key: &str,
        args: &[(&str, &str)],
    ) -> Result<String, String> {
        self.message(locale, key)
            .ok_or_else(|| format!("unknown message ID {key}"))?
            .render(args)
    }
}

static RUNTIME: OnceLock<LivingCatalog> = OnceLock::new();

fn runtime() -> &'static LivingCatalog {
    RUNTIME.get_or_init(|| {
        if let Some(path) = env::var_os("SEALED_LOCALE_DIR").map(PathBuf::from) {
            match LivingCatalog::from_directory(&path) {
                Ok(catalog) => return catalog,
                Err(error) => {
                    eprintln!("Invalid living locale pack; using embedded catalogs: {error}")
                }
            }
        }
        LivingCatalog::embedded().expect("validated shipped living localization")
    })
}

pub(crate) fn living_text(locale: Locale, key: &'static str) -> &'static str {
    runtime().text(locale, key)
}

pub(crate) fn living_format(locale: Locale, key: &str, args: &[(&str, &str)]) -> String {
    runtime().format(locale, key, args).unwrap_or_else(|error| {
        eprintln!("Localization formatting error: {key}: {error}");
        key.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_keys_are_paired_and_default_strings_unchanged() {
        let catalog = LivingCatalog::embedded().unwrap();
        assert_eq!(catalog.english.len(), catalog.indonesian.len());
        assert_eq!(
            catalog.text(Locale::English, "living.save.ok"),
            "Saved the settlement."
        );
        assert_eq!(
            catalog.text(Locale::Indonesian, "living.save.ok"),
            "Desa tersimpan."
        );
        assert_eq!(
            catalog.text(Locale::Indonesian, "unknown.key"),
            "unknown.key"
        );
    }
    #[test]
    fn external_partial_indonesian_falls_back_to_english() {
        let catalog = LivingCatalog::with_overrides(
            Some(r#"{"living.load.ok":"Loaded"}"#),
            Some(r#"{"living.save.ok":"Tersimpan"}"#),
        )
        .unwrap();
        assert_eq!(
            catalog.text(Locale::Indonesian, "living.save.ok"),
            "Tersimpan"
        );
        assert_eq!(catalog.text(Locale::Indonesian, "living.load.ok"), "Loaded");
        assert_eq!(
            catalog.text(Locale::English, "living.save.ok"),
            "Saved the settlement."
        );
    }
    #[test]
    fn named_parameters_validate_and_do_not_interpret_inserted_data() {
        let catalog = LivingCatalog::with_overrides(
            None,
            Some(r#"{"living.save.error":"{error} : gagal {{simpan}}"}"#),
        )
        .unwrap();
        assert_eq!(
            catalog
                .format(
                    Locale::Indonesian,
                    "living.save.error",
                    &[("error", "{literal}")]
                )
                .unwrap(),
            "{literal} : gagal {simpan}"
        );
        assert!(
            catalog
                .format(Locale::English, "living.save.error", &[])
                .is_err()
        );
        assert!(
            catalog
                .format(
                    Locale::English,
                    "living.save.error",
                    &[("error", "x"), ("error", "y")]
                )
                .is_err()
        );
        assert!(
            catalog
                .format(Locale::English, "living.save.error", &[("other", "x")])
                .is_err()
        );
        assert!(catalog.format(Locale::English, "unknown", &[]).is_err());
        assert_eq!(
            catalog.text(Locale::English, "living.save.error"),
            "living.save.error"
        );
    }
    #[test]
    fn templates_can_reorder_named_values_and_reject_terminal_controls() {
        let a = Message::parse("{actor}: {count}").unwrap();
        let b = Message::parse("{count} untuk {actor}").unwrap();
        assert_eq!(a.parameters, b.parameters);
        assert_eq!(
            b.render(&[("actor", "Luo Xiu"), ("count", "2")]).unwrap(),
            "2 untuk Luo Xiu"
        );
        assert!(Message::parse("\u{1b}[2J").is_err());
        assert!(Message::parse("\r overwrite").is_err());
    }

    #[test]
    fn invalid_overrides_reject_whole_pack() {
        for invalid in [
            r#"{"living.save.error":"missing parameter"}"#,
            r#"{"living.save.error":"{wrong}"}"#,
            r#"{"living.save.ok":"{unexpected}"}"#,
            r#"{"living.save.ok":"oops }"}"#,
            r#"{"living.save.ok":"oops {"}"#,
            r#"{"living.save.ok":"a","living.save.ok":"b"}"#,
            r#"{"unknown":"x"}"#,
            r#"{"living.save.ok":""}"#,
        ] {
            assert!(
                LivingCatalog::with_overrides(None, Some(invalid)).is_err(),
                "{invalid}"
            );
        }
        assert!(
            LivingCatalog::with_overrides(
                Some(r#"{"living.save.ok":"ok"}"#),
                Some(r#"{"bad":"no"}"#)
            )
            .is_err()
        );
    }
    #[test]
    fn bounded_utf8_files_and_missing_pack_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        assert!(LivingCatalog::from_directory(dir.path()).is_err());
        let file = dir.path().join("living_id.json");
        std::fs::write(&file, [0xff]).unwrap();
        assert!(LivingCatalog::from_directory(dir.path()).is_err());
        std::fs::write(&file, vec![b' '; MAX_PACK_BYTES as usize + 1]).unwrap();
        assert!(LivingCatalog::from_directory(dir.path()).is_err());
        std::fs::write(&file, r#"{"living.save.ok":"Tersimpan"}"#).unwrap();
        assert_eq!(
            LivingCatalog::from_directory(dir.path())
                .unwrap()
                .text(Locale::Indonesian, "living.save.ok"),
            "Tersimpan"
        );
        assert!(LivingCatalog::from_directory(Path::new("relative")).is_err());
    }
}
