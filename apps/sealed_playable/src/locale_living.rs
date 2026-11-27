//! Runtime language resources for living-world presentation (C3.6).
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
const MAX_RENDER_BYTES: usize = 64 * 1024;

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
        // Most templates use fewer than ten arguments. Check without a per-call
        // tree allocation, and bound expansion even for a repeated placeholder.
        if args.len() != self.parameters.len() {
            return Err("missing or unknown parameters".into());
        }
        for (index, &(name, _)) in args.iter().enumerate() {
            if !self.parameters.contains(name) {
                return Err(format!("unknown parameter {name}"));
            }
            if args[..index].iter().any(|(prior, _)| *prior == name) {
                return Err(format!("duplicate parameter {name}"));
            }
        }
        let mut result = String::new();
        for part in &self.parts {
            let value = match part {
                Part::Literal(value) => value.as_str(),
                Part::Parameter(name) => args
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| *value)
                    .ok_or_else(|| format!("missing parameter {name}"))?,
            };
            if result.len().saturating_add(value.len()) > MAX_RENDER_BYTES {
                return Err("rendered message exceeds byte limit".into());
            }
            result.push_str(value);
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

pub(crate) fn read_optional(path: &Path) -> Result<Option<String>, String> {
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

    /// English uses singular for exactly one. Indonesian nouns use the
    /// invariant other form. If the selected Indonesian template is absent,
    /// choose the correct English number form, not a mixed-language plural.
    pub fn plural(
        &self,
        locale: Locale,
        one: &str,
        other: &str,
        count: u64,
        args: &[(&str, &str)],
    ) -> Result<String, String> {
        let singular = self
            .english
            .get(one)
            .ok_or_else(|| format!("unknown message ID {one}"))?;
        let multiple = self
            .english
            .get(other)
            .ok_or_else(|| format!("unknown message ID {other}"))?;
        if singular.parameters != multiple.parameters {
            return Err("plural parameter contract mismatch".into());
        }
        if locale == Locale::Indonesian
            && let Some(message) = self.indonesian.get(other)
        {
            return message.render(args);
        }
        if count == 1 {
            singular.render(args)
        } else {
            multiple.render(args)
        }
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

/// Prefer an explicit absolute directory; otherwise use a pack next to the
/// executable. Never auto-discover catalogs from the current working directory.
pub(crate) fn runtime_directory() -> Option<PathBuf> {
    if let Some(path) = env::var_os("SEALED_LOCALE_DIR") {
        return Some(PathBuf::from(path));
    }
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("locales")))
        .filter(|dir| dir.is_dir())
}

static RUNTIME: OnceLock<LivingCatalog> = OnceLock::new();

fn runtime() -> &'static LivingCatalog {
    RUNTIME.get_or_init(|| {
        if let Some(path) = runtime_directory()
            && (!path.is_absolute()
                || !path.is_dir()
                || path.join("living_en.json").exists()
                || path.join("living_id.json").exists())
        {
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

pub(crate) fn living_plural(
    locale: Locale,
    one: &str,
    other: &str,
    count: u64,
    args: &[(&str, &str)],
) -> String {
    runtime()
        .plural(locale, one, other, count, args)
        .unwrap_or_else(|error| {
            eprintln!("Localization plural formatting error: {other}: {error}");
            other.to_owned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_presentation_ids_and_context_entries_are_valid() {
        let catalog = LivingCatalog::embedded().unwrap();
        let raw = include_str!("../locales/living_context.json");
        let context: serde_json::Value = serde_json::from_str(raw).unwrap();
        let entries = context.as_object().unwrap();
        assert_eq!(entries.len(), catalog.english.len());
        for key in catalog.english.keys() {
            assert!(entries.contains_key(key), "{key}");
        }
        let mut active = BTreeSet::new();
        for source in [
            include_str!("living_ascii.rs"),
            include_str!("living_pixel.rs"),
        ] {
            for token in source.split("#[cfg(test)]").next().unwrap().split('"') {
                if token.starts_with("living.")
                    && token.chars().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'
                    })
                {
                    assert!(catalog.english.contains_key(token), "unregistered {token}");
                    active.insert(token);
                }
            }
        }
        for (key, entry) in entries {
            assert_eq!(
                entry["active"].as_bool(),
                Some(active.contains(key.as_str())),
                "{key}"
            );
            if let Some(params) = entry["parameters"].as_object() {
                let declared: BTreeSet<_> = params.keys().cloned().collect();
                assert_eq!(declared, catalog.english[key].parameters, "{key}");
            }
        }
    }

    #[test]
    fn plural_uses_actual_fallback_language_and_rendering_is_bounded() {
        let other = "living.ascii.outcome_label.template.you_took_herbs_herbs_from_actor";
        let one = "living.ascii.outcome_label.template.you_took_herbs_herbs_from_actor.one";
        let catalog = LivingCatalog::embedded().unwrap();
        let args = [("herbs", "1"), ("actor", "Luo Xiu")];
        assert_eq!(
            catalog
                .plural(Locale::English, one, other, 1, &args)
                .unwrap(),
            "You took 1 herb from Luo Xiu."
        );
        assert_eq!(
            catalog
                .plural(
                    Locale::English,
                    one,
                    other,
                    2,
                    &[("herbs", "2"), ("actor", "Luo Xiu")]
                )
                .unwrap(),
            "You took 2 herbs from Luo Xiu."
        );
        assert!(
            catalog
                .plural(Locale::Indonesian, one, other, 1, &args)
                .unwrap()
                .contains("herba")
        );
        let fallback = LivingCatalog::with_overrides(None, Some("{}")).unwrap();
        assert_eq!(
            fallback
                .plural(Locale::Indonesian, one, other, 1, &args)
                .unwrap(),
            "You took 1 herb from Luo Xiu."
        );
        let huge = "x".repeat(MAX_RENDER_BYTES);
        assert!(
            Message::parse("{x}{x}")
                .unwrap()
                .render(&[("x", &huge)])
                .is_err()
        );
    }

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
