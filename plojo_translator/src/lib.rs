#[macro_use]
extern crate lazy_static;

use dictionary::Dictionary;
use diff::translation_diff;
use plojo_core::{Command, Stroke, Translator};
use serde::Deserialize;
use std::{error::Error, hash::Hash};

mod dictionary;
mod diff;

/// A dictionary entry. It could be a command, in which case it is passed directly to the
/// dispatcher. Otherwise it is something that pertains to text, which is parsed here in translator
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
enum Translation {
    Text(Vec<Text>),
    Command {
        cmds: Vec<Command>,
        text_after: Option<Vec<Text>>,
        suppress_space_before: bool,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Clone, Hash, Eq, Deserialize)]
enum Text {
    // text literal that can be upper/lower cased
    Lit(String),
    // unknown strokes always printed in all caps
    UnknownStroke(Stroke),
    // a string that can be attached to the previous and/or next word
    Attached {
        // the text itself
        text: String,
        // if it should be attached to the next word
        joined_next: bool,
        /// Whether or not to apply orthography rules and whether to attach to the next word
        joined_prev: AttachedType,
        /// whether or not to carry the capitalization state to the word following this
        carry_capitalization: bool,
    },
    // glued strokes only attach to other glued strokes
    Glued(String),
    // changes the state for suppressing space, capitalizing, etc. the next word
    StateAction(StateAction),
    // text actions can only affect the text before it
    TextAction(TextAction),
}

#[derive(Debug, PartialEq, Clone, Hash, Eq, Deserialize)]
enum AttachedType {
    ApplyOrthography,
    AttachOnly,
    DoNotAttach,
}

impl Translation {
    /// Convert translation into text, ignoring commands
    fn as_text(&self) -> Vec<Text> {
        match self {
            Translation::Text(ref text) => text.clone(),
            Translation::Command { text_after, .. } => text_after.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Hash, Eq, Deserialize)]
enum StateAction {
    ForceCapitalize,
    SameCase(bool), // apply all upper (true) or lower (false) case
    Clear,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize)]
enum TextAction {
    CapitalizePrev,
    SuppressSpacePrev,
    SameCasePrev(bool), // apply all upper (true) or lower (false) case
}

/// The standard translator is very similar in feature to Plover and other CAT software.
///
/// It translates a stroke into a command by looking up the stroke in a dictionary. It maintains a
/// history of pressed strokes and tries to look up the longest stroke in the dictionary. If any
/// stroke in retrospective_add_space is pressed, the `add_space_insert` stroke will be inserted into
/// before the previous (undoable) stroke
#[derive(Debug, PartialEq)]
pub struct StandardTranslator {
    prev_strokes: Vec<Stroke>,
    dict: Dictionary,
    retrospective_add_space: Vec<Stroke>,
    add_space_insert: Option<Stroke>,
    space_after: bool,
}

// most number of strokes to stroke in prev_strokes; limits undo to this many strokes
const MAX_STROKE_BUFFER: usize = 50;
// only pass a certain number of strokes to be translated
const MAX_TRANSLATION_STROKE_LEN: usize = 10;

/// Check whether the translation is non empty text
/// Used to determine where to add retrospective space
fn is_text(translation: Translation) -> bool {
    match translation {
        Translation::Command { text_after, .. } => {
            if let Some(text_after) = text_after {
                is_text(Translation::Text(text_after))
            } else {
                false
            }
        }
        Translation::Text(texts) => {
            // check if at least one is non empty text
            for text in texts {
                match text {
                    Text::UnknownStroke(_) => return true,
                    Text::Attached { text, .. } | Text::Glued(text) | Text::Lit(text) => {
                        if !text.is_empty() {
                            return true;
                        }
                    }
                    Text::TextAction(_) | Text::StateAction(_) => continue,
                }
            }
            false
        }
    }
}

impl StandardTranslator {
    /// Creates a translator that takes the raw dictionary string from one or more dictionaries. The
    /// dictionaries further down in the list can override the earlier dictionaries.
    ///
    /// The starting strokes will be added to the stroke list when the translator is created.
    ///
    /// It has strokes for retroactivly adding a space and the space stroke that is actually added
    ///
    /// # Panics
    /// Panics if retrospective_add_space is none empty but add_space_insert is None
    pub fn new(
        raw_dicts: Vec<String>,
        starting_strokes: Vec<Stroke>,
        retrospective_add_space: Vec<Stroke>,
        add_space_insert: Option<Stroke>,
        space_after: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let dict = Dictionary::new(raw_dicts)?;
        // if there are retrospective add space strokes, there must be a space stroke
        if !retrospective_add_space.is_empty() {
            assert!(add_space_insert.is_some());
        }

        Ok(Self {
            prev_strokes: starting_strokes,
            dict,
            retrospective_add_space,
            add_space_insert,
            space_after,
        })
    }
}

impl Translator for StandardTranslator {
    fn translate(&mut self, stroke: Stroke) -> Vec<Command> {
        if self.prev_strokes.len() > MAX_STROKE_BUFFER {
            self.prev_strokes.remove(0);
        }

        // translate only latest strokes
        let start = if self.prev_strokes.len() > MAX_TRANSLATION_STROKE_LEN {
            self.prev_strokes.len() - MAX_TRANSLATION_STROKE_LEN
        } else {
            0
        };

        let old_translations = self.dict.translate(&self.prev_strokes[start..]);

        // add a space if necessary
        if self.retrospective_add_space.contains(&stroke) {
            let mut index = self.prev_strokes.len();
            // find the first undoable stroke (from the back)
            for s in self.prev_strokes.iter().rev() {
                index -= 1;
                let translated = self.dict.translate(&[s.clone()]);
                if translated.into_iter().any(is_text) {
                    break;
                }
            }

            // add a space
            if let Some(space) = self.add_space_insert.clone() {
                self.prev_strokes.insert(index, space);
            }
        } else {
            self.prev_strokes.push(stroke);
        }

        let new_translations = self.dict.translate(&self.prev_strokes[start..]);

        translation_diff(&old_translations, &new_translations, self.space_after)
    }

    fn undo(&mut self) -> Vec<Command> {
        let old_translations = self.dict.translate(&self.prev_strokes);

        // keep on removing strokes as long as they are the same (when diffed)
        while !self.prev_strokes.is_empty() {
            self.prev_strokes.pop();
            let new_translations = self.dict.translate(&self.prev_strokes);
            let diff = translation_diff(&old_translations, &new_translations, self.space_after);
            if diff != vec![Command::NoOp] {
                return diff;
            }
        }

        return vec![Command::NoOp];
    }

    /// Handle a command for the translator.
    ///
    /// Valid commands are:
    /// - "clear_prev_strokes": Clears the stroke buffer
    /// - "toggle_space_after": Toggles between space after and space before
    fn handle_command(&mut self, command: String) {
        match command.as_ref() {
            "clear_prev_strokes" => {
                // remove every stroke before the last, because that stroke triggered this command
                // and the last stroke could have text_after text that needs to be preserved
                let mut v = Vec::with_capacity(MAX_STROKE_BUFFER);
                if let Some(last) = self.prev_strokes.pop() {
                    v.push(last);
                }
                self.prev_strokes = v;
            }
            "toggle_space_after" => {
                self.space_after = !self.space_after;
            }
            _c => eprintln!("[WARN]: the standard translator cannot handle {:?}", _c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_text() {
        assert_eq!(
            is_text(Translation::Text(vec![Text::Lit("hello".to_owned())])),
            true
        );
        assert_eq!(
            is_text(Translation::Text(vec![Text::Glued("s".to_owned())])),
            true
        );
        assert_eq!(
            is_text(Translation::Text(vec![Text::StateAction(
                StateAction::ForceCapitalize
            )])),
            false
        );
        assert_eq!(
            is_text(Translation::Text(vec![Text::TextAction(
                TextAction::CapitalizePrev
            )])),
            false
        );
        assert_eq!(
            is_text(Translation::Command {
                cmds: vec![],
                text_after: None,
                suppress_space_before: false,
            }),
            false
        );
        assert_eq!(
            is_text(Translation::Command {
                cmds: vec![Command::NoOp],
                text_after: Some(vec![Text::StateAction(StateAction::ForceCapitalize)]),
                suppress_space_before: false,
            }),
            false
        );
        assert_eq!(
            is_text(Translation::Command {
                cmds: vec![Command::NoOp],
                text_after: Some(vec![]),
                suppress_space_before: false,
            }),
            false
        );
    }
}
