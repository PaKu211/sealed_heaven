//! Presentation-only localization. Canonical IDs, numbers and payloads are never translated.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Stable labels required by the desktop surface, in both shipped dictionaries.
pub const REQUIRED_KEYS: &[&str] = &[
    "app.title",
    "controls.pause",
    "controls.resume",
    "controls.step",
    "controls.save",
    "controls.load",
    "controls.reset",
    "controls.language",
    "controls.select",
    "controls.camera_reset",
    "help.controls",
    "help.inspector",
    "help.needs",
    "help.travel",
    "help.save",
    "inspector.title",
    "inspector.npc",
    "inspector.position",
    "inspector.needs",
    "inspector.goal",
    "inspector.stage",
    "inspector.task",
    "inspector.inventory",
    "inspector.stocks",
    "inspector.owner",
    "inspector.events",
    "inspector.tick",
    "inspector.checkpoint",
    "inspector.digest",
    "inspector.enabled",
    "inspector.mode",
    "inspector.none",
    "inspector.id",
    "inspector.definition",
    "inspector.quantity",
    "inspector.class",
    "inspector.payload",
    "inspector.kind",
    "inspector.site",
    "inspector.edge",
    "inspector.grid",
    "inspector.progress",
    "inspector.home",
    "inspector.holder",
    "status.ready",
    "status.saved",
    "status.loaded",
    "status.reset",
    "status.error",
    "status.paused",
    "status.running",
    "locale.en",
    "locale.id",
    "value.yes",
    "value.no",
    "mode.1",
    "mode.2",
    "goal.0",
    "goal.1",
    "goal.2",
    "goal.3",
    "goal.4",
    "goal.5",
    "stage.1",
    "stage.2",
    "stage.3",
    "stage.4",
    "stage.5",
    "stage.6",
    "stage.7",
    "stage.8",
    "event.1",
    "event.2",
    "event.3",
    "event.4",
    "event.5",
    "event.6",
    "event.7",
    "need.0",
    "need.1",
    "need.2",
    "need.3",
    "need.4",
    "owner.1",
    "owner.2",
    "owner.3",
    "owner.4",
    "owner.5",
    "class.1",
    "class.2",
    "error.drive",
    "error.save",
    "error.load",
    "error.reset",
    "error.catalog",
    "error.checkpoint",
    "error.oversized",
    "error.io",
    "validation.utf8",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Locale {
    #[default]
    English,
    Indonesian,
}

impl Locale {
    /// Unknown codes deliberately select the English source language.
    pub fn from_code(code: &str) -> Self {
        if code.trim().eq_ignore_ascii_case("id") {
            Self::Indonesian
        } else {
            Self::English
        }
    }
}

#[derive(Clone, Debug)]
pub struct Catalog {
    english: BTreeMap<String, String>,
    indonesian: BTreeMap<String, String>,
}

impl Catalog {
    /// C0.5 adds a namespaced overlay without changing the C0.4 dictionaries.
    pub fn agency() -> Result<Self, String> {
        let mut catalog = Self::load()?;
        let en = parse_dictionary("c05/en", include_str!("../locales/c05_en.json"))?;
        let id = parse_dictionary("c05/id", include_str!("../locales/c05_id.json"))?;
        if !en.keys().eq(id.keys()) {
            return Err("c05 dictionary key parity mismatch".into());
        }
        for key in AGENCY_KEYS {
            if !en.contains_key(*key) {
                return Err(format!("c05 missing required key {key}"));
            }
        }
        for (key, value) in en {
            if catalog.english.insert(key.clone(), value).is_some() {
                return Err(format!("c05 shadowed key {key}"));
            }
        }
        for (key, value) in id {
            if catalog.indonesian.insert(key.clone(), value).is_some() {
                return Err(format!("c05 shadowed key {key}"));
            }
        }
        Ok(catalog)
    }
    pub fn load() -> Result<Self, String> {
        Self::from_json(
            include_str!("../locales/en.json"),
            include_str!("../locales/id.json"),
        )
    }

    /// Validate external dictionaries before publishing a usable catalog.
    /// This also permits validation tests without altering shipped resources.
    pub fn from_json(english: &str, indonesian: &str) -> Result<Self, String> {
        let english = parse_dictionary("en", english)?;
        let indonesian = parse_dictionary("id", indonesian)?;
        for (code, dictionary) in [("en", &english), ("id", &indonesian)] {
            for key in REQUIRED_KEYS {
                if !dictionary.contains_key(*key) {
                    return Err(format!("{code}: missing required key {key:?}"));
                }
            }
        }
        if !english.keys().eq(indonesian.keys()) {
            let only_en: Vec<_> = english
                .keys()
                .filter(|key| !indonesian.contains_key(*key))
                .collect();
            let only_id: Vec<_> = indonesian
                .keys()
                .filter(|key| !english.contains_key(*key))
                .collect();
            return Err(format!(
                "dictionary key parity mismatch: only en={only_en:?}, only id={only_id:?}"
            ));
        }
        Ok(Self {
            english,
            indonesian,
        })
    }

    /// Prefer the selected language, then English, then the diagnostic key itself.
    /// Values are returned verbatim; callers append canonical data without translating it.
    pub fn text(&self, locale: Locale, key: &str) -> String {
        let selected = match locale {
            Locale::English => &self.english,
            Locale::Indonesian => &self.indonesian,
        };
        selected
            .get(key)
            .or_else(|| self.english.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }
}

pub const AGENCY_KEYS: &[&str] = &[
    "c05.title",
    "c05.player",
    "c05.settlement",
    "c05.healer",
    "c05.patch",
    "c05.shrine",
    "c05.herbs",
    "c05.source",
    "c05.delivered",
    "c05.inventory",
    "c05.target",
    "c05.range",
    "c05.far",
    "c05.talk",
    "c05.gather",
    "c05.deliver",
    "c05.controls",
    "c05.ready",
    "c05.queued",
    "c05.queue_full",
    "c05.resolved",
    "c05.request",
    "c05.complete",
    "c05.unmet",
    "c05.npc_alive",
    "c05.queue",
    "c05.moved",
    "c05.requested",
    "c05.gathered",
    "c05.delivered_ok",
    "c05.rejected",
    "c05.r_out_of_range",
    "c05.r_terrain",
    "c05.r_occupied",
    "c05.r_empty",
    "c05.r_request",
    "c05.r_complete",
    "c05.r_npc",
    "c05.r_overflow",
    "c05.pending",
    "c05.follow",
    "c05.saved_prefix",
    "c05.cell",
    "c05.entry",
];

struct Dictionary(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for Dictionary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DictionaryVisitor;

        impl<'de> Visitor<'de> for DictionaryVisitor {
            type Value = Dictionary;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a flat JSON object of nonempty string values")
            }

            fn visit_map<A>(self, mut entries: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut dictionary = BTreeMap::new();
                while let Some(key) = entries.next_key::<String>()? {
                    // Check decoded keys before insertion: a normal map would silently
                    // overwrite duplicates, including keys written with JSON escapes.
                    if dictionary.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate key {key:?}")));
                    }
                    let value = entries.next_value::<String>()?;
                    if value.trim().is_empty() {
                        return Err(de::Error::custom(format!("empty value for key {key:?}")));
                    }
                    dictionary.insert(key, value);
                }
                Ok(Dictionary(dictionary))
            }
        }

        deserializer.deserialize_map(DictionaryVisitor)
    }
}

fn parse_dictionary(code: &str, source: &str) -> Result<BTreeMap<String, String>, String> {
    serde_json::from_str::<Dictionary>(source)
        .map(|dictionary| dictionary.0)
        .map_err(|error| format!("{code}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGLISH: &str = include_str!("../locales/en.json");
    const INDONESIAN: &str = include_str!("../locales/id.json");

    #[test]
    fn c05_overlay_covers_both_languages_without_changing_old_labels() {
        let catalog = Catalog::agency().unwrap();
        let old = Catalog::load().unwrap();
        for key in REQUIRED_KEYS {
            for locale in [Locale::English, Locale::Indonesian] {
                assert_eq!(catalog.text(locale, key), old.text(locale, key));
            }
        }
        for key in AGENCY_KEYS {
            for locale in [Locale::English, Locale::Indonesian] {
                assert_ne!(catalog.text(locale, key), *key);
            }
        }
    }

    fn changed_dictionary(
        source: &str,
        change: impl FnOnce(&mut BTreeMap<String, String>),
    ) -> String {
        let mut dictionary = parse_dictionary("test", source).unwrap();
        change(&mut dictionary);
        serde_json::to_string(&dictionary).unwrap()
    }

    #[test]
    fn shipped_dictionaries_parse_and_cover_identical_required_keys() {
        let catalog = Catalog::load().unwrap();
        assert!(catalog.english.keys().eq(catalog.indonesian.keys()));
        assert_eq!(catalog.english.len(), REQUIRED_KEYS.len());
        let unique: std::collections::BTreeSet<_> = REQUIRED_KEYS.iter().collect();
        assert_eq!(unique.len(), REQUIRED_KEYS.len());
        for key in REQUIRED_KEYS {
            for locale in [Locale::English, Locale::Indonesian] {
                let text = catalog.text(locale, key);
                assert!(!text.trim().is_empty(), "{locale:?}: {key}");
                assert_ne!(text, *key);
            }
        }
    }

    #[test]
    fn duplicate_keys_are_rejected_before_map_conversion_in_either_language() {
        for duplicate in [
            r#"{"app.title":"First","app.title":"Second"}"#,
            r#"{"app.title":"First","app.\u0074itle":"Second"}"#,
        ] {
            assert!(
                Catalog::from_json(duplicate, INDONESIAN)
                    .unwrap_err()
                    .contains("en: duplicate key")
            );
            assert!(
                Catalog::from_json(ENGLISH, duplicate)
                    .unwrap_err()
                    .contains("id: duplicate key")
            );
        }
    }

    #[test]
    fn empty_and_whitespace_values_are_rejected_in_either_language() {
        for value in ["", " \t\n", "\u{2003}"] {
            let invalid = changed_dictionary(ENGLISH, |dictionary| {
                dictionary.insert("app.title".into(), value.into());
            });
            assert!(
                Catalog::from_json(&invalid, INDONESIAN)
                    .unwrap_err()
                    .contains("empty value")
            );
            assert!(
                Catalog::from_json(ENGLISH, &invalid)
                    .unwrap_err()
                    .contains("empty value")
            );
        }
    }

    #[test]
    fn missing_required_keys_are_rejected_even_if_both_languages_omit_them() {
        let incomplete = changed_dictionary(ENGLISH, |dictionary| {
            dictionary.remove("controls.pause");
        });
        for (english, indonesian) in [
            (incomplete.as_str(), INDONESIAN),
            (ENGLISH, incomplete.as_str()),
            (incomplete.as_str(), incomplete.as_str()),
        ] {
            assert!(
                Catalog::from_json(english, indonesian)
                    .unwrap_err()
                    .contains("missing required key \"controls.pause\"")
            );
        }
    }

    #[test]
    fn extra_keys_must_also_have_parity() {
        let extra = changed_dictionary(ENGLISH, |dictionary| {
            dictionary.insert("extra.label".into(), "Extra".into());
        });
        assert!(
            Catalog::from_json(&extra, INDONESIAN)
                .unwrap_err()
                .contains("parity mismatch")
        );
        assert!(
            Catalog::from_json(ENGLISH, &extra)
                .unwrap_err()
                .contains("parity mismatch")
        );
        assert!(Catalog::from_json(&extra, &extra).is_ok());
    }

    #[test]
    fn malformed_nonflat_nonstring_and_trailing_json_are_rejected() {
        for invalid in [
            "{",
            "[]",
            "null",
            r#"{"app.title":{"nested":"value"}}"#,
            r#"{"app.title":1}"#,
            r#"{"app.title":null}"#,
            r#"{"app.title":"ok"} {}"#,
        ] {
            assert!(Catalog::from_json(invalid, INDONESIAN).is_err());
        }
    }

    #[test]
    fn locale_codes_fall_back_to_english() {
        for code in ["en", "EN", "", "unknown", "fr", "id-ID"] {
            assert_eq!(Locale::from_code(code), Locale::English);
        }
        for code in ["id", "ID", " id "] {
            assert_eq!(Locale::from_code(code), Locale::Indonesian);
        }
        let catalog = Catalog::load().unwrap();
        assert_eq!(
            catalog.text(Locale::from_code("unknown"), "controls.pause"),
            "Pause"
        );
    }

    #[test]
    fn missing_translation_falls_back_to_english_and_unknown_key_is_diagnostic() {
        let mut catalog = Catalog::load().unwrap();
        // Valid shipped catalogs are complete; simulate a future partial runtime catalog.
        catalog.indonesian.remove("controls.pause");
        assert_eq!(catalog.text(Locale::Indonesian, "controls.pause"), "Pause");
        for locale in [Locale::English, Locale::Indonesian] {
            assert_eq!(catalog.text(locale, "unknown.key"), "unknown.key");
        }
    }

    #[test]
    fn indonesian_and_non_ascii_utf8_survive_verbatim() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.text(Locale::Indonesian, "controls.pause"), "Jeda");
        assert_eq!(
            catalog.text(Locale::Indonesian, "validation.utf8"),
            "Teks UTF-8: café — selamat datang di dunia NPC."
        );
        let raw = r#"{"text":"é 日本語 🏠","escaped":"caf\u00e9"}"#;
        let parsed = parse_dictionary("test", raw).unwrap();
        assert_eq!(parsed["text"], "é 日本語 🏠");
        assert_eq!(parsed["escaped"], "café");
    }

    #[test]
    fn switching_language_only_reads_catalog_and_never_formats_canonical_data() {
        let catalog = Catalog::load().unwrap();
        let before = catalog.english.clone();
        for _ in 0..20 {
            for locale in [Locale::Indonesian, Locale::English] {
                assert_eq!(catalog.text(locale, "000123"), "000123");
                assert_eq!(catalog.text(locale, "deadbeef"), "deadbeef");
                assert!(!catalog.text(locale, "inspector.digest").is_empty());
            }
        }
        assert_eq!(catalog.english, before);
    }
}
