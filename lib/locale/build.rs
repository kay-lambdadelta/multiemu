use csv::Reader;
use proc_macro2::Span;
use quote::quote;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::PathBuf};
use syn::{Fields, Ident, Variant};

#[derive(Debug, Deserialize)]
struct CountryRecord {
    #[serde(rename = "ISO3166-1-Alpha-2")]
    alpha_2_code: String,
    #[serde(rename = "ISO3166-1-Alpha-3")]
    alpha_3_code: String,
}

#[derive(Debug, Deserialize)]
struct LanguageRecord {
    alpha2: Option<String>,
    #[serde(rename = "alpha3-b")]
    alpha3: String,
}

fn main() {
    generate_3166();
    generate_639();
}

pub fn generate_3166() {
    // Read CSV
    let file = File::open("data/country-codes.csv").expect("Could not open CSV");
    let mut reader = Reader::from_reader(file);

    // Construct variants
    let mut variants_2 = Vec::new();
    let mut variants_3 = Vec::new();
    let mut mappings_2to3 = HashMap::new();
    let mut mappings_3to2 = HashMap::new();

    for result in reader.deserialize() {
        let record: CountryRecord = result.expect("CSV parse error");

        let ident_str = record.alpha_2_code.to_uppercase();
        let ident2 = Ident::new(&ident_str, Span::call_site());

        variants_2.push(Variant {
            attrs: Vec::new(),
            ident: ident2.clone(),
            fields: Fields::Unit,
            discriminant: None,
        });

        let ident_str = record.alpha_3_code.to_uppercase();
        let ident3 = Ident::new(&ident_str, Span::call_site());

        variants_3.push(Variant {
            attrs: Vec::new(),
            ident: ident3.clone(),
            fields: Fields::Unit,
            discriminant: None,
        });

        mappings_2to3.insert(ident2.clone(), ident3.clone());
        mappings_3to2.insert(ident3, ident2);
    }

    let derives: Vec<_> = [
        "Copy",
        "Clone",
        "Hash",
        "PartialEq",
        "Eq",
        "PartialOrd",
        "Ord",
        "Debug",
    ]
    .into_iter()
    .map(|ident| Ident::new(ident, Span::call_site()))
    .collect();

    let arms_2to3 = mappings_2to3.iter().map(|(k, v)| {
        quote! { Self::#k => Iso3166Alpha3::#v, }
    });
    let arms_3to2 = mappings_3to2.iter().map(|(k, v)| {
        quote! { Self::#k => Iso3166Alpha2::#v, }
    });
    let display_arms_2 = variants_2.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { Self::#ident => #s, }
    });
    let fromstr_arms_2 = variants_2.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { #s => Ok(Self::#ident), }
    });
    let display_arms_3 = variants_3.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { Self::#ident => #s, }
    });
    let fromstr_arms_3 = variants_3.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { #s => Ok(Self::#ident), }
    });

    // Build the enum AST
    let file = quote! {
        #[derive(#(#derives),*)]
        pub enum Iso3166Alpha2 {
            #(#variants_2),*
        }

        impl Iso3166Alpha2 {
            pub fn to_alpha3(&self) -> Iso3166Alpha3 {
                match self {
                    #(#arms_2to3)*
                }
            }
        }

        impl std::fmt::Display for Iso3166Alpha2 {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    #(#display_arms_2)*
                };
                write!(f, "{}", s)
            }
        }
        impl std::str::FromStr for Iso3166Alpha2 {
            type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    #(#fromstr_arms_2)*
                    _ => Err(format!("unknown ISO3166 code: {}", s)),
                }
            }
        }

        impl serde::Serialize for Iso3166Alpha2 {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
        impl<'de> serde::Deserialize<'de> for Iso3166Alpha2 {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }

        #[derive(#(#derives),*)]
        pub enum Iso3166Alpha3 {
            #(#variants_3),*
        }

        impl Iso3166Alpha3 {
            pub fn to_alpha2(&self) -> Iso3166Alpha2 {
                match self {
                    #(#arms_3to2)*
                }
            }
        }

        impl std::fmt::Display for Iso3166Alpha3 {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    #(#display_arms_3)*
                };
                write!(f, "{}", s)
            }
        }
        impl std::str::FromStr for Iso3166Alpha3 {
            type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    #(#fromstr_arms_3)*
                    _ => Err(format!("unknown ISO3166 code: {}", s)),
                }
            }
        }

        impl serde::Serialize for Iso3166Alpha3 {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
        impl<'de> serde::Deserialize<'de> for Iso3166Alpha3 {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };

    // Convert AST to formatted Rust code using prettyplease
    let code = prettyplease::unparse(&syn::parse2(file).unwrap());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest_path = out_dir.join("iso3166.rs");
    std::fs::write(dest_path, code).expect("Failed to write generated Rust file");
}

pub fn generate_639() {
    // Read CSV
    let file = File::open("data/language-codes-full.csv").expect("Could not open CSV");
    let mut reader = Reader::from_reader(file);

    // Construct variants
    let mut variants_2 = Vec::new();
    let mut variants_3 = Vec::new();
    let mut mappings_2to3 = HashMap::new();
    let mut mappings_3to2 = HashMap::new();

    for result in reader.deserialize() {
        let record: LanguageRecord = result.expect("CSV parse error");

        // Skip reserved
        if record.alpha3 == "qaa-qtz" {
            continue;
        }

        let ident2 = record.alpha2.map(|alpha2| {
            let ident_str = alpha2.to_uppercase();
            let ident2 = Ident::new(&ident_str, Span::call_site());

            variants_2.push(Variant {
                attrs: Vec::new(),
                ident: ident2.clone(),
                fields: Fields::Unit,
                discriminant: None,
            });

            ident2
        });

        let ident_str = record.alpha3.to_uppercase();
        let ident3 = Ident::new(&ident_str, Span::call_site());

        variants_3.push(Variant {
            attrs: Vec::new(),
            ident: ident3.clone(),
            fields: Fields::Unit,
            discriminant: None,
        });

        if let Some(ident2) = ident2 {
            mappings_2to3.insert(ident2.clone(), ident3.clone());
            mappings_3to2.insert(ident3, ident2);
        }
    }

    let derives: Vec<_> = [
        "Copy",
        "Clone",
        "Hash",
        "PartialEq",
        "Eq",
        "PartialOrd",
        "Ord",
        "Debug",
    ]
    .into_iter()
    .map(|ident| Ident::new(ident, Span::call_site()))
    .collect();

    let arms_2to3 = mappings_2to3.iter().map(|(k, v)| {
        quote! { Self::#k => Iso639Alpha3::#v, }
    });
    let arms_3to2 = mappings_3to2.iter().map(|(k, v)| {
        quote! { Self::#k => Some(Iso639Alpha2::#v), }
    });
    let display_arms_2 = variants_2.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { Self::#ident => #s, }
    });
    let fromstr_arms_2 = variants_2.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { #s => Ok(Self::#ident), }
    });
    let display_arms_3 = variants_3.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { Self::#ident => #s, }
    });
    let fromstr_arms_3 = variants_3.iter().map(|variant| {
        let ident = variant.ident.clone();
        let s = ident.to_string().to_lowercase();
        quote! { #s => Ok(Self::#ident), }
    });

    // Build the enum AST
    let file = quote! {
        #[derive(#(#derives),*)]
        pub enum Iso639Alpha2 {
            #(#variants_2),*
        }

        impl Iso639Alpha2 {
            pub fn to_alpha3(&self) -> Iso639Alpha3 {
                match self {
                    #(#arms_2to3)*
                }
            }
        }

        impl std::fmt::Display for Iso639Alpha2 {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    #(#display_arms_2)*
                };
                write!(f, "{}", s)
            }
        }
        impl std::str::FromStr for Iso639Alpha2 {
            type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    #(#fromstr_arms_2)*
                    _ => Err(format!("unknown IS639 code: {}", s)),
                }
            }
        }

        impl serde::Serialize for Iso639Alpha2 {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
        impl<'de> serde::Deserialize<'de> for Iso639Alpha2 {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }

        #[derive(#(#derives),*)]
        pub enum Iso639Alpha3 {
            #(#variants_3),*
        }
        impl Iso639Alpha3 {
            pub fn to_alpha2(&self) -> Option<Iso639Alpha2> {
                match self {
                    #(#arms_3to2)*
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for Iso639Alpha3 {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    #(#display_arms_3)*
                };
                write!(f, "{}", s)
            }
        }
        impl std::str::FromStr for Iso639Alpha3 {
            type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    #(#fromstr_arms_3)*
                    _ => Err(format!("unknown IS639 code: {}", s)),
                }
            }
        }

        impl serde::Serialize for Iso639Alpha3 {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
        impl<'de> serde::Deserialize<'de> for Iso639Alpha3 {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };

    let code = prettyplease::unparse(&syn::parse2(file).unwrap());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest_path = out_dir.join("iso639.rs");
    std::fs::write(dest_path, code).expect("Failed to write generated Rust file");
}
