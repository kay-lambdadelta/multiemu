use isolang::Language;
use std::collections::HashSet;
use std::str::FromStr;

fn get_data_in_parentheses(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut stack = Vec::new();

    for (i, c) in input.chars().enumerate() {
        match c {
            '(' => {
                stack.push(i);
            }
            ')' => {
                if let Some(start) = stack.pop() {
                    let substring = &input[start + 1..i];
                    result.push(substring.to_string());
                }
            }
            _ => {}
        }
    }

    result
}

pub struct NameMetadataExtractor {
    pub languages: HashSet<Language>,
}

impl FromStr for NameMetadataExtractor {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut languages = HashSet::new();

        // Split the string into parts based on parentheses
        let parts = get_data_in_parentheses(s);

        for part in parts {
            let part = part.to_lowercase();
            let part = part.trim().split(',');

            for part in part {
                if let Some(language) = Language::from_639_1(part) {
                    languages.insert(language);
                }

                // Region to default locale
                match part {
                    "usa" => {
                        languages.insert(Language::from_639_1("en").unwrap());
                    }
                    "united kingdom" => {
                        languages.insert(Language::from_639_1("en").unwrap());
                    }
                    "japan" => {
                        languages.insert(Language::from_639_1("ja").unwrap());
                    }
                    _ => {}
                }
            }
        }

        Ok(NameMetadataExtractor { languages })
    }
}
