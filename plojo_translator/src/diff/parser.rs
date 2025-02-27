use crate::{AttachedType, StateAction, Text, TextAction};
use orthography::apply_orthography;
use regex::Regex;
use std::char;

mod orthography;

lazy_static! {
    // whether a translation contains only digits or the center dash
    // although the regex will mark "-" as a number, such a stroke is not possible
    static ref NUMBER_TRANSLATION_REGEX: Regex = Regex::new(r"^[0-9\-]+$").unwrap();
    // whether a translation contains only digits, in which case it will be glued
    static ref NUMBERS_ONLY_REGEX: Regex = Regex::new(r"^[0-9]+$").unwrap();
}

const SPACE: char = ' ';

#[derive(Debug, Default)]
struct State {
    suppress_space: bool,
    force_capitalize: bool,
    prev_is_glued: bool,
    force_same_case: Option<bool>,
}

/// Converts translations into their string representation by adding spaces in between words and
/// applying text actions. Has an option to insert spaces after words instead of before.
///
/// A state of the spaces/capitalization is kept as it loops over the Texts to build the string.
/// StateActions change that state
pub(super) fn parse_translation(translations: Vec<Text>, space_after: bool) -> String {
    // current state
    let mut state: State = Default::default();
    let mut str = String::new();

    for t in translations {
        let next_word;
        let mut next_state: State = Default::default();

        match t {
            Text::Lit(text) => {
                next_word = text.clone();
                // glue it if it is a number stroke
                if NUMBERS_ONLY_REGEX.is_match(&next_word) {
                    next_state.prev_is_glued = true;
                    if state.prev_is_glued {
                        state.suppress_space = true;
                    }
                }
            }
            Text::UnknownStroke(stroke) => {
                let raw_stroke = stroke.to_raw();
                // glue it if it is a number stroke
                if NUMBER_TRANSLATION_REGEX.is_match(&raw_stroke) {
                    // remove the hyphen
                    next_word = raw_stroke.replace("-", "");
                    next_state.prev_is_glued = true;
                    if state.prev_is_glued {
                        state.suppress_space = true;
                    }
                } else {
                    next_word = raw_stroke;
                }
            }
            Text::Attached {
                text,
                joined_next,
                joined_prev,
                carry_capitalization,
            } => {
                next_word = text.clone();
                if joined_next {
                    next_state.suppress_space = true;
                }
                if carry_capitalization {
                    // carry on the capitalization state to the next word
                    next_state.force_capitalize = state.force_capitalize;
                    next_state.force_same_case = state.force_same_case;
                    // don't capitalize this word
                    state.force_capitalize = false;
                }

                // don't apply orthography if previous stroke suppressed the next space
                // this is so suppress space can output a suffix literally (without ortho rule)
                if !state.suppress_space {
                    match joined_prev {
                        AttachedType::DoNotAttach => {
                            // do nothing
                        }
                        AttachedType::AttachOnly => {
                            state.suppress_space = true;
                        }
                        AttachedType::ApplyOrthography => {
                            state.suppress_space = true;
                            // find last none alpha character
                            let index = str.rfind(|c: char| !c.is_alphabetic()).map_or(0, |i| {
                                // we want the index of the next char
                                let mut char_size = 0;
                                // find number of bytes of that char and increment by that much
                                for (idx, c) in str.char_indices() {
                                    if idx == i {
                                        char_size = c.len_utf8();
                                        break;
                                    }
                                }
                                // rfind found the index, so it should exist in char_indices
                                assert!(char_size > 0);
                                i + char_size
                            });
                            // find the last word and apply orthography rule with the suffix
                            if index < str.len() {
                                let new_word = apply_orthography(&str[index..], &text);
                                // replace that word with the new (orthography'ed) one
                                str = str[..index].to_string() + &new_word;
                            } else {
                                // there was no last word, directly add the text
                                str = str + &text;
                            }
                            state = next_state;
                            continue;
                        }
                    };
                }
            }
            Text::Glued(text) => {
                next_word = text.clone();
                next_state.prev_is_glued = true;
                if state.prev_is_glued {
                    state.suppress_space = true;
                }
            }
            Text::StateAction(action) => {
                match action {
                    StateAction::ForceCapitalize => {
                        state.force_capitalize = true;
                    }
                    StateAction::SameCase(b) => {
                        state.force_same_case = Some(b);
                    }
                    StateAction::Clear => {
                        // reset formatting state
                        state = Default::default();
                    }
                }
                continue;
            }
            Text::TextAction(action) => {
                str = perform_text_action(&str, action);
                continue;
            }
        }

        if !state.suppress_space {
            str.push(SPACE);
        }

        let mut word = next_word;
        if state.force_capitalize {
            word = word_change_first_letter(word);
        }
        if let Some(b) = state.force_same_case {
            word = if b {
                word.to_uppercase()
            } else {
                word.to_lowercase()
            };
        }
        str.push_str(&word);

        state = next_state;
    }

    // put space after if it is configured to do so
    if space_after && !str.is_empty() {
        // remove the leading space if there is any
        if let Some(maybe_space) = str.chars().next() {
            if maybe_space == SPACE {
                str.remove(0);
            }
        }
        if !state.suppress_space {
            str.push(SPACE);
        }
    }

    str
}

/// Forces the first letter of a string to be uppercase
fn word_change_first_letter(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Find the index in the text after the last space
/// This index is 0 if there is no whitespace, and text.len() if the last char is a whitespace
fn find_last_word_space(text: &str) -> usize {
    if let Some(i) = text.rfind(char::is_whitespace) {
        // add 1 to remove the space
        // whitespace takes up 1 byte, so adding 1 is safe here
        i + 1
    } else {
        // no whitespace, so everything must be a word
        0
    }
}

// chars (besides alphanumeric) that are considered part of a word
// This is used for deciding what is a word when capitalizing the previous word
const WORD_CHARS: [char; 2] = ['-', '_'];

/// Find the index of the last word by looking for a non alphanumeric or non word character
fn find_last_word(text: &str) -> usize {
    // find the last non-alphanumeric (nor hyphen) character
    if let Some(i) = text.rfind(|c| !(char::is_alphanumeric(c) || WORD_CHARS.contains(&c))) {
        // size of whatever char was before the word
        // unwrap is safe because we found the index `i` with rfind
        let char_size = text[i..].chars().next().unwrap().to_string().len();
        // add to get to the next char (the actual word)
        i + char_size
    } else {
        // no whitespace, so everything must be a word
        0
    }
}

fn perform_text_action(text: &str, action: TextAction) -> String {
    match action {
        TextAction::SuppressSpacePrev => {
            let mut new_str = text.to_string();
            let index = find_last_word_space(&text);
            // find the last word and see if there is a space before it
            if index > 0 && text.get(index - 1..index) == Some(" ") {
                // remove the space (this is safe because we checked the index above)
                new_str.remove(index - 1);
            }
            new_str
        }
        TextAction::CapitalizePrev => {
            let index = find_last_word(&text);
            let word = text[index..].to_string();
            let capitalized = word_change_first_letter(word);
            text[..index].to_string() + &capitalized
        }
        TextAction::SameCasePrev(b) => {
            let index = find_last_word(&text);
            let word = text[index..].to_string();
            let changed_case = if b {
                word.to_uppercase()
            } else {
                word.to_lowercase()
            };
            text[..index].to_string() + &changed_case
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StateAction, TextAction};
    use plojo_core::Stroke;

    fn translation_diff_space_after(t: Vec<Text>) -> String {
        parse_translation(t, false)
    }

    #[test]
    fn test_parse_empty() {
        let translated = translation_diff_space_after(vec![]);

        assert_eq!(translated, "");
    }

    #[test]
    fn test_parse_basic() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("hello".to_string()),
            Text::Lit("hi".to_string()),
        ]);

        assert_eq!(translated, " hello hi");
    }

    #[test]
    fn test_parse_text_actions() {
        let translated = translation_diff_space_after(vec![
            Text::Attached {
                text: "".to_string(),
                joined_next: true,
                joined_prev: AttachedType::AttachOnly,
                carry_capitalization: false,
            },
            Text::StateAction(StateAction::ForceCapitalize),
            Text::Lit("hello".to_string()),
            Text::Lit("hi".to_string()),
            Text::StateAction(StateAction::ForceCapitalize),
            Text::Lit("FOo".to_string()),
            Text::Lit("bar".to_string()),
            Text::Lit("baZ".to_string()),
            Text::Attached {
                text: "".to_string(),
                joined_next: true,
                joined_prev: AttachedType::AttachOnly,
                carry_capitalization: false,
            },
            Text::Lit("NICE".to_string()),
            Text::Attached {
                text: "".to_string(),
                joined_next: true,
                joined_prev: AttachedType::AttachOnly,
                carry_capitalization: false,
            },
            Text::Lit("".to_string()),
            Text::Lit("well done".to_string()),
        ]);

        assert_eq!(translated, "Hello hi FOo bar baZNICE well done");
    }

    #[test]
    fn test_parse_prev_word_text_actions() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("hi".to_string()),
            Text::TextAction(TextAction::CapitalizePrev),
            Text::TextAction(TextAction::CapitalizePrev),
            Text::Lit("FOo".to_string()),
            Text::Lit("bar".to_string()),
            Text::TextAction(TextAction::SuppressSpacePrev),
            Text::TextAction(TextAction::CapitalizePrev),
            Text::Lit("hello".to_string()),
            Text::Lit("Hi a".to_string()),
            Text::TextAction(TextAction::CapitalizePrev),
            Text::StateAction(StateAction::ForceCapitalize),
            Text::Lit("nice".to_string()),
            Text::UnknownStroke(Stroke::new("TP-TDZ")),
            Text::TextAction(TextAction::SuppressSpacePrev),
            Text::Lit("nice".to_string()),
            Text::Attached {
                text: "".to_string(),
                joined_next: true,
                joined_prev: AttachedType::AttachOnly,
                carry_capitalization: false,
            },
            Text::Lit("another".to_string()),
        ]);

        assert_eq!(translated, " Hi FOobar hello Hi A NiceTP-TDZ niceanother");
    }

    #[test]
    fn test_parse_line_start() {
        let translated = translation_diff_space_after(vec![
            Text::Attached {
                text: "".to_string(),
                joined_next: true,
                joined_prev: AttachedType::AttachOnly,
                carry_capitalization: false,
            },
            Text::StateAction(StateAction::ForceCapitalize),
            Text::Lit("hello".to_string()),
            Text::Lit("hi".to_string()),
        ]);

        assert_eq!(translated, "Hello hi");
    }

    #[test]
    fn test_parse_glued() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("hello".to_string()),
            Text::Glued("hi".to_string()),
            Text::Glued("hi".to_string()),
            Text::Lit("foo".to_string()),
            Text::Glued("two".to_string()),
            Text::Glued("three".to_string()),
        ]);

        assert_eq!(translated, " hello hihi foo twothree");
    }

    #[test]
    fn test_word_change_first_letter() {
        assert_eq!(word_change_first_letter("hello".to_owned()), "Hello");
        assert_eq!(word_change_first_letter("".to_owned()), "");
        assert_eq!(word_change_first_letter("Hello".to_owned()), "Hello");
    }

    #[test]
    fn test_unicode() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("hi".to_string()),
            Text::Lit("hello".to_string()),
            Text::Lit("𐀀".to_string()),
            Text::TextAction(TextAction::SuppressSpacePrev),
            Text::Lit("©aa".to_string()),
            Text::TextAction(TextAction::CapitalizePrev),
            Text::TextAction(TextAction::SuppressSpacePrev),
        ]);

        assert_eq!(translated, " hi hello𐀀©Aa");
    }

    #[test]
    fn test_double_space() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("hello".to_string()),
            Text::Attached {
                text: " ".to_string(),
                joined_next: true,
                joined_prev: AttachedType::ApplyOrthography,
                carry_capitalization: false,
            },
            Text::Attached {
                text: " ".to_string(),
                joined_next: true,
                joined_prev: AttachedType::ApplyOrthography,
                carry_capitalization: false,
            },
        ]);

        assert_eq!(translated, " hello  ");
    }

    #[test]
    fn test_find_last_word_space() {
        assert_eq!(find_last_word_space("hello world"), 6);
        assert_eq!(find_last_word_space(" world"), 1);
        assert_eq!(find_last_word_space("test "), 5);
        assert_eq!(find_last_word_space("nospace"), 0);
        assert_eq!(find_last_word_space(" there are many words"), 16);
    }

    #[test]
    fn test_find_last_word() {
        assert_eq!(find_last_word("hello world"), 6);
        assert_eq!(find_last_word(" world"), 1);
        assert_eq!(find_last_word("test "), 5);
        assert_eq!(find_last_word("not:this-that"), 4);
        assert_eq!(find_last_word("THE Under_score"), 4);
    }

    #[test]
    fn test_perform_text_action() {
        assert_eq!(
            perform_text_action("foo bar", TextAction::SuppressSpacePrev),
            "foobar"
        );
        assert_eq!(
            perform_text_action(" hello", TextAction::CapitalizePrev),
            " Hello"
        );
        assert_eq!(
            perform_text_action(" there are many words", TextAction::CapitalizePrev),
            " there are many Words"
        );
        assert_eq!(
            perform_text_action(" no previous word ", TextAction::CapitalizePrev),
            " no previous word "
        );
        assert_eq!(
            perform_text_action(" ∅∅byteboundary", TextAction::CapitalizePrev),
            " ∅∅Byteboundary"
        );
        assert_eq!(
            // This weird character becomes 2 S's when capitalized
            perform_text_action(" ßweird_char", TextAction::CapitalizePrev),
            " SSweird_char"
        );
        assert_eq!(
            perform_text_action(" (symbol", TextAction::CapitalizePrev),
            " (Symbol"
        );
        assert_eq!(
            perform_text_action(" !symbol-hyphen", TextAction::CapitalizePrev),
            " !Symbol-hyphen"
        );
    }

    #[test]
    fn test_carry_capitalization() {
        let translated = translation_diff_space_after(vec![
            Text::Lit("fairy".to_string()),
            Text::StateAction(StateAction::ForceCapitalize),
            Text::Attached {
                text: "s".to_string(),
                joined_next: false,
                joined_prev: AttachedType::ApplyOrthography,
                carry_capitalization: true,
            },
            Text::Attached {
                text: "b".to_string(),
                joined_next: true,
                joined_prev: AttachedType::DoNotAttach,
                carry_capitalization: true,
            },
            Text::Lit("hi".to_string()),
        ]);

        assert_eq!(translated, " fairies bHi");
    }

    #[test]
    fn test_space_after_basic() {
        let translated = parse_translation(
            vec![
                Text::Lit("hello".to_string()),
                Text::StateAction(StateAction::ForceCapitalize),
                Text::Attached {
                    text: "a".to_string(),
                    joined_next: false,
                    joined_prev: AttachedType::AttachOnly,
                    carry_capitalization: false,
                },
            ],
            true,
        );

        assert_eq!(translated, "helloA ");
    }

    #[test]
    fn test_space_after_suppress_space() {
        let translated = parse_translation(
            vec![
                Text::Lit("hello".to_string()),
                Text::Lit("world".to_string()),
                Text::Attached {
                    text: "".to_string(),
                    joined_next: true,
                    joined_prev: AttachedType::DoNotAttach,
                    carry_capitalization: false,
                },
            ],
            true,
        );

        assert_eq!(translated, "hello world ");
    }

    #[test]
    fn test_space_after_glued() {
        let translated = parse_translation(
            vec![
                Text::Glued("a".to_string()),
                Text::Glued("b".to_string()),
                Text::Glued("c".to_string()),
            ],
            true,
        );

        assert_eq!(translated, "abc ");
    }

    #[test]
    fn test_space_after_empty() {
        let translated = parse_translation(vec![], true);

        assert_eq!(translated, "");
    }

    #[test]
    fn test_alpha_orthograhy() {
        let translated = parse_translation(
            vec![
                Text::Attached {
                    text: "©".to_string(),
                    joined_next: true,
                    joined_prev: AttachedType::DoNotAttach,
                    carry_capitalization: false,
                },
                Text::Lit("model".to_string()),
                Text::Attached {
                    text: "ed".to_string(),
                    joined_next: false,
                    joined_prev: AttachedType::ApplyOrthography,
                    carry_capitalization: false,
                },
            ],
            false,
        );

        assert_eq!(translated, " ©modeled");
    }

    #[test]
    fn test_force_same_case() {
        let translated = parse_translation(
            vec![
                Text::StateAction(StateAction::SameCase(true)),
                Text::StateAction(StateAction::ForceCapitalize),
                Text::Lit("hello".to_string()),
                // force same case should override force capitalize
                Text::StateAction(StateAction::ForceCapitalize),
                Text::StateAction(StateAction::SameCase(false)),
                Text::Attached {
                    text: "(".to_string(),
                    joined_next: true,
                    joined_prev: AttachedType::DoNotAttach,
                    carry_capitalization: true,
                },
                Text::Lit("NASA".to_string()),
                Text::Lit("hi".to_string()),
                Text::TextAction(TextAction::CapitalizePrev),
                Text::TextAction(TextAction::SameCasePrev(true)),
                Text::Lit("aLL_cAPs".to_string()),
                // force same case prev should override force capitalize prev
                Text::TextAction(TextAction::CapitalizePrev),
                Text::TextAction(TextAction::SameCasePrev(false)),
            ],
            false,
        );

        assert_eq!(translated, " HELLO (nasa HI all_caps");
    }
}
