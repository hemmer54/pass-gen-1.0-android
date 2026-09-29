use std::fmt;

const LOWERCASE: &str = "abcdefghijkmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ";
const DIGITS: &str = "23456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}:,.?";
const PASSPHRASE_WORDS: &[&str] = &[
    "amber", "anchor", "apple", "apricot", "atlas", "badge", "beacon", "birch", "breeze", "cactus",
    "canyon", "cedar", "cinder", "cobalt", "comet", "coral", "cricket", "dawn", "delta", "ember",
    "falcon", "fern", "fjord", "forest", "galaxy", "harbor", "hazel", "iris", "island", "jade",
    "juniper", "lantern", "maple", "meadow", "meteor", "midnight", "mint", "mocha", "north",
    "oasis", "olive", "orbit", "pebble", "pepper", "pixel", "planet", "plum", "quartz", "raven",
    "river", "rocket", "saffron", "shadow", "signal", "silver", "spruce", "star", "stone", "stone",
    "summit", "sunset", "tiger", "topaz", "trail", "violet", "walnut", "willow", "winter",
    "zenith",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    RandomPassword,
    Passphrase,
}

#[derive(Debug, Clone, Copy)]
pub struct GeneratorSettings {
    pub mode: GenerationMode,
    pub length: usize,
    pub words: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
    pub avoid_repeats: bool,
    pub separator: char,
}

impl Default for GeneratorSettings {
    fn default() -> Self {
        Self {
            mode: GenerationMode::RandomPassword,
            length: 20,
            words: 4,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
            avoid_repeats: false,
            separator: '-',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationError {
    EmptyCharacterSet,
    RandomnessUnavailable(String),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCharacterSet => formatter.write_str("Select at least one character group"),
            Self::RandomnessUnavailable(error) => {
                write!(formatter, "Secure randomness is unavailable: {error}")
            }
        }
    }
}

impl std::error::Error for GenerationError {}

pub fn generate(settings: GeneratorSettings) -> Result<String, GenerationError> {
    match settings.mode {
        GenerationMode::RandomPassword => generate_password(settings),
        GenerationMode::Passphrase => generate_passphrase(settings),
    }
}

pub fn character_set_size(settings: GeneratorSettings) -> usize {
    build_character_set(settings).chars().count()
}

pub fn entropy_bits(settings: GeneratorSettings) -> f32 {
    match settings.mode {
        GenerationMode::RandomPassword => {
            let size = character_set_size(settings) as f32;
            if size > 1.0 {
                settings.length as f32 * size.log2()
            } else {
                0.0
            }
        }
        GenerationMode::Passphrase => {
            settings.words as f32 * (PASSPHRASE_WORDS.len() as f32).log2()
        }
    }
}

pub fn strength_label(bits: f32) -> &'static str {
    match bits {
        value if value < 45.0 => "Weak",
        value if value < 70.0 => "Moderate",
        value if value < 100.0 => "Strong",
        _ => "Excellent",
    }
}

fn generate_password(settings: GeneratorSettings) -> Result<String, GenerationError> {
    let characters = build_character_set(settings);
    let characters = characters.as_bytes();
    if characters.is_empty() {
        return Err(GenerationError::EmptyCharacterSet);
    }

    let mut result = String::with_capacity(settings.length);
    let mut previous = None;
    for _ in 0..settings.length.clamp(8, 64) {
        let character = loop {
            let candidate = characters[random_index(characters.len())?] as char;
            if settings.avoid_repeats && previous == Some(candidate) && characters.len() > 1 {
                continue;
            }
            break candidate;
        };
        result.push(character);
        previous = Some(character);
    }
    Ok(result)
}

fn generate_passphrase(settings: GeneratorSettings) -> Result<String, GenerationError> {
    let mut words = Vec::with_capacity(settings.words.clamp(3, 8));
    for _ in 0..settings.words.clamp(3, 8) {
        words.push(PASSPHRASE_WORDS[random_index(PASSPHRASE_WORDS.len())?]);
    }
    Ok(words.join(&settings.separator.to_string()))
}

fn build_character_set(settings: GeneratorSettings) -> String {
    let mut characters = String::new();
    if settings.lowercase {
        characters.push_str(LOWERCASE);
    }
    if settings.uppercase {
        characters.push_str(UPPERCASE);
    }
    if settings.digits {
        characters.push_str(DIGITS);
    }
    if settings.symbols {
        characters.push_str(SYMBOLS);
    }
    if settings.exclude_ambiguous {
        characters.retain(|character| {
            !matches!(
                character,
                'i' | 'l' | 'o' | 'I' | 'O' | '0' | '1' | '|' | '`' | '\''
            )
        });
    }
    characters
}

fn random_index(length: usize) -> Result<usize, GenerationError> {
    debug_assert!(length > 0 && length <= 256);
    let limit = 256 - (256 % length);
    let mut byte = [0u8; 1];
    loop {
        getrandom::fill(&mut byte)
            .map_err(|error| GenerationError::RandomnessUnavailable(error.to_string()))?;
        if (byte[0] as usize) < limit {
            return Ok(byte[0] as usize % length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_password_matches_requested_length() {
        let settings = GeneratorSettings::default();
        let password = generate(settings).expect("secure random generation should work");
        assert_eq!(password.chars().count(), settings.length);
    }

    #[test]
    fn disabled_groups_are_not_used() {
        let settings = GeneratorSettings {
            uppercase: false,
            lowercase: true,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
            ..GeneratorSettings::default()
        };
        let password = generate(settings).expect("lowercase generation should work");
        assert!(password
            .chars()
            .all(|character| LOWERCASE.contains(character)));
    }

    #[test]
    fn passphrase_has_requested_number_of_words() {
        let settings = GeneratorSettings {
            mode: GenerationMode::Passphrase,
            words: 6,
            ..GeneratorSettings::default()
        };
        let passphrase = generate(settings).expect("passphrase generation should work");
        assert_eq!(passphrase.split('-').count(), 6);
    }

    #[test]
    fn no_character_groups_is_rejected() {
        let settings = GeneratorSettings {
            uppercase: false,
            lowercase: false,
            digits: false,
            symbols: false,
            ..GeneratorSettings::default()
        };
        assert_eq!(generate(settings), Err(GenerationError::EmptyCharacterSet));
    }
}
