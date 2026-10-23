// SPDX-License-Identifier: AGPL-3.0-or-later

//! Short session codes: `NNN-WORD-WORD`.
//!
//! `NNN` is a public slot number. The two words are the PAKE password and are
//! never written to the DHT. Each list has 2048 words, so the password carries
//! 22 bits. That is only enough because a host throws the code away after a
//! few wrong guesses.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::error::DiscoveryError;

/// Which word list a code uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// English list, derived from BIP-39.
    En,
    /// Turkish list. Display uses Turkish casing (`i` → `İ`, `ı` → `I`).
    Tr,
}

/// A generated or parsed short code. Words are stored in lowercase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortCode {
    /// Public slot, `0..=999`.
    pub slot: u16,
    /// List the words were drawn from.
    pub language: Language,
    /// Two list words. They may be the same word twice.
    pub words: [String; 2],
}

impl ShortCode {
    /// Draw a code from the operating system CSPRNG.
    pub fn generate(language: Language) -> Result<Self, DiscoveryError> {
        let list = word_list(language)?;
        let slot = u16::try_from(random_below(1000)?).map_err(|_| DiscoveryError::Rng)?;
        let len = u32::try_from(list.len())
            .map_err(|_| DiscoveryError::Wordlist("word list is too long"))?;
        let first_at = usize::try_from(random_below(len)?).map_err(|_| DiscoveryError::Rng)?;
        let second_at = usize::try_from(random_below(len)?).map_err(|_| DiscoveryError::Rng)?;
        let first = list[first_at].to_string();
        let second = list[second_at].to_string();
        Ok(Self {
            slot,
            language,
            words: [first, second],
        })
    }

    /// Render `428-KARTAL-DENİZ` style text.
    pub fn format(&self) -> String {
        format!(
            "{:03}-{}-{}",
            self.slot,
            display_word(&self.words[0], self.language),
            display_word(&self.words[1], self.language)
        )
    }
}

/// Parse a formatted code. Matching is case-insensitive for the selected list.
pub fn parse_code(text: &str, language: Language) -> Result<ShortCode, DiscoveryError> {
    let parts: Vec<&str> = text.trim().split('-').collect();
    if parts.len() != 3 || parts[0].len() != 3 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
        return Err(DiscoveryError::BadCode);
    }
    let slot = parts[0]
        .parse::<u16>()
        .map_err(|_| DiscoveryError::BadCode)?;
    if slot > 999 {
        return Err(DiscoveryError::SlotOutOfRange);
    }
    let list = word_list(language)?;
    let first = lookup(list, parts[1], language)?;
    let second = lookup(list, parts[2], language)?;
    Ok(ShortCode {
        slot,
        language,
        words: [first, second],
    })
}

/// The 2048 words for `language`.
pub fn word_list(language: Language) -> Result<&'static [&'static str], DiscoveryError> {
    match language {
        Language::En => cached(include_str!("../wordlists/en.txt"), Language::En, &EN),
        Language::Tr => cached(include_str!("../wordlists/tr.txt"), Language::Tr, &TR),
    }
}

fn cached(
    text: &'static str,
    language: Language,
    slot: &'static OnceLock<Result<Vec<&'static str>, &'static str>>,
) -> Result<&'static [&'static str], DiscoveryError> {
    match slot.get_or_init(|| parse_wordlist(text, language)) {
        Ok(words) => Ok(words.as_slice()),
        Err(reason) => Err(DiscoveryError::Wordlist(reason)),
    }
}

static EN: OnceLock<Result<Vec<&'static str>, &'static str>> = OnceLock::new();
static TR: OnceLock<Result<Vec<&'static str>, &'static str>> = OnceLock::new();

fn parse_wordlist(
    text: &'static str,
    language: Language,
) -> Result<Vec<&'static str>, &'static str> {
    let mut words = Vec::new();
    let mut seen = HashSet::new();
    let mut folded = HashSet::new();
    for raw in text.lines() {
        let word = raw.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }
        if !charset_ok(word, language) {
            return Err("word has characters outside the list alphabet");
        }
        if !seen.insert(word) {
            return Err("duplicate word");
        }
        if language == Language::Tr && !folded.insert(fold_tr(word)) {
            return Err("two words collide after Turkish diacritics are folded");
        }
        words.push(word);
    }
    if words.len() != 2048 {
        return Err("word list must contain 2048 words");
    }
    Ok(words)
}

fn charset_ok(word: &str, language: Language) -> bool {
    match language {
        Language::En => word.chars().all(|c| c.is_ascii_lowercase()),
        Language::Tr => word
            .chars()
            .all(|c| matches!(c, 'a'..='z' | 'ç' | 'ğ' | 'ı' | 'ö' | 'ş' | 'ü')),
    }
}

fn fold_tr(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            'ı' => 'i',
            'ö' => 'o',
            'ü' => 'u',
            'ç' => 'c',
            'ş' => 's',
            'ğ' => 'g',
            other => other,
        })
        .collect()
}

fn lookup(list: &[&str], token: &str, language: Language) -> Result<String, DiscoveryError> {
    let folded: String = match language {
        Language::En => token.chars().map(|c| c.to_ascii_lowercase()).collect(),
        Language::Tr => token.chars().map(tr_lower).collect(),
    };
    list.iter()
        .find(|word| **word == folded)
        .map(|word| (*word).to_string())
        .ok_or(DiscoveryError::BadCode)
}

fn display_word(word: &str, language: Language) -> String {
    match language {
        Language::En => word.to_ascii_uppercase(),
        Language::Tr => word.chars().map(tr_upper).collect(),
    }
}

fn tr_upper(c: char) -> char {
    match c {
        'i' => 'İ',
        'ı' => 'I',
        'ç' => 'Ç',
        'ğ' => 'Ğ',
        'ö' => 'Ö',
        'ş' => 'Ş',
        'ü' => 'Ü',
        'a'..='z' => char::from(b'A' + (c as u8 - b'a')),
        other => other,
    }
}

fn tr_lower(c: char) -> char {
    match c {
        'İ' => 'i',
        'I' => 'ı',
        'Ç' => 'ç',
        'Ğ' => 'ğ',
        'Ö' => 'ö',
        'Ş' => 'ş',
        'Ü' => 'ü',
        'A'..='Z' => char::from(b'a' + (c as u8 - b'A')),
        other => other,
    }
}

fn random_below(max_exclusive: u32) -> Result<u32, DiscoveryError> {
    let span = u32::MAX - (u32::MAX % max_exclusive);
    loop {
        let mut buf = [0u8; 4];
        getrandom::getrandom(&mut buf).map_err(|_| DiscoveryError::Rng)?;
        let value = u32::from_le_bytes(buf);
        if value < span {
            return Ok(value % max_exclusive);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_2048_unique_words_without_turkish_diacritic_pairs() {
        let english = word_list(Language::En).expect("en");
        let turkish = word_list(Language::Tr).expect("tr");
        assert_eq!(english.len(), 2048);
        assert_eq!(turkish.len(), 2048);
        assert_eq!(english[0], "abandon");
        assert_eq!(english[2047], "zoo");
        assert!(turkish.contains(&"çınar"));
        assert!(turkish.contains(&"acı"));
        assert!(!turkish.contains(&"açı"));
        assert!(!turkish.contains(&"âlem"));
    }

    #[test]
    fn format_and_parse_roundtrip_with_turkish_casing() {
        let turkish = word_list(Language::Tr).expect("tr");
        let with_i = turkish.iter().find(|word| word.contains('i')).expect("i");
        let with_dotless = turkish.iter().find(|word| word.contains('ı')).expect("ı");
        let code = ShortCode {
            slot: 8,
            language: Language::Tr,
            words: [(*with_i).to_string(), (*with_dotless).to_string()],
        };
        let formatted = code.format();
        assert!(formatted.starts_with("008-"));
        assert!(formatted.contains('İ') || !with_i.contains('i'));
        if with_i.contains('i') {
            assert!(formatted.contains('İ'));
        }
        if with_dotless.contains('ı') {
            assert!(!formatted.contains('ı'));
        }
        assert_eq!(parse_code(&formatted, Language::Tr).expect("parse"), code);

        let english = ShortCode {
            slot: 428,
            language: Language::En,
            words: ["abandon".to_string(), "zoo".to_string()],
        };
        assert_eq!(english.format(), "428-ABANDON-ZOO");
        assert_eq!(
            parse_code("428-abandon-zoo", Language::En).expect("parse"),
            english
        );
    }

    #[test]
    fn generated_codes_vary_and_stay_in_range() {
        let mut slots = Vec::new();
        for _ in 0..16 {
            let code = ShortCode::generate(Language::En).expect("generate");
            assert!(code.slot <= 999);
            let list = word_list(Language::En).expect("en");
            assert!(list.contains(&code.words[0].as_str()));
            assert!(list.contains(&code.words[1].as_str()));
            assert_eq!(
                parse_code(&code.format(), Language::En).expect("parse"),
                code
            );
            slots.push(code.slot);
        }
        assert!(slots.iter().any(|slot| *slot != slots[0]));
    }
}
