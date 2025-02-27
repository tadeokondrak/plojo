use crate::Translation;
use plojo_core::Stroke;
use std::collections::HashMap;
use std::error::Error;
use std::iter::FromIterator;

mod load;
mod translate;

type DictEntry = (Stroke, Translation);

#[derive(Debug, PartialEq)]
pub struct Dictionary {
    strokes: HashMap<Stroke, Translation>,
}

impl Dictionary {
    /// Create a new dictionary from raw JSON strings. Each string represents a dictionary, with
    /// each dictionaries being able to overwrite any dictionary entry before it
    pub fn new(raw_dicts: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut entries = vec![];
        for raw_dict in raw_dicts {
            entries.append(&mut load::load_dicts(&raw_dict)?);
        }

        Ok(entries.into_iter().collect())
    }

    fn lookup(&self, strokes: &[Stroke]) -> Option<Translation> {
        // combine strokes with a `/` between them
        let combined = strokes
            .iter()
            .map(|s| s.clone().to_raw())
            .collect::<Vec<_>>()
            .join("/");

        self.strokes.get(&Stroke::new(&combined)).cloned()
    }

    pub(super) fn translate(&self, strokes: &[Stroke]) -> Vec<Translation> {
        translate::translate_strokes(self, strokes)
    }
}

impl FromIterator<DictEntry> for Dictionary {
    fn from_iter<T: IntoIterator<Item = DictEntry>>(iter: T) -> Self {
        let mut hashmap: HashMap<Stroke, Translation> = HashMap::new();
        for (stroke, translations) in iter {
            hashmap.insert(stroke, translations);
        }

        Dictionary { strokes: hashmap }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Text, Translation};

    #[test]
    fn dictionary_overwrite() {
        let raw_dict1 = r#"
            {
                "H-L": "hello",
                "WORLD": "world"
            }
        "#
        .to_string();
        let raw_dict2 = r#"
            {
                "WORLD": "something else"
            }
        "#
        .to_string();

        let dict = Dictionary::new(vec![raw_dict1, raw_dict2]).unwrap();
        assert_eq!(
            dict.lookup(&[Stroke::new("WORLD")]).unwrap(),
            Translation::Text(vec![Text::Lit("something else".to_string())])
        );
    }
}
