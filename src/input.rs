use std::str::FromStr;
use std::error::Error;
use std::collections::BTreeMap;

use tokei::Languages;
use tokei::{LanguageType, Language};

type LanguageMap = BTreeMap<LanguageType, Language>;

macro_rules! supported_formats {
    ($(
        ($name:ident, $feature:expr, $variant:ident) =>
            |$parse_param:ident| $parse_kode:block,
            |$print_param:ident| $print_kode:block,
    )+) => (
        /// Supported serialization formats.
        ///
        /// To enable all formats compile with the `all` feature.
        #[derive(Debug)]
        pub enum Format {
            $(
                #[cfg(feature = $feature)] $variant
            ),+

            // TODO: Allow adding format at runtime when used as a lib?
        }

        impl Format {
            pub fn supported() -> &'static [&'static str] {
                &[
                    $(
                        #[cfg(feature = $feature)] stringify!($name)
                    ),+
                ]
            }

            pub fn all() -> &'static [&'static str] {
                &[
                    $( stringify!($name) ),+
                ]
            }

            pub fn not_supported() -> &'static [&'static str] {
                &[
                    $(
                        #[cfg(not(feature = $feature))] stringify!($name)
                    ),+
                ]
            }

            pub fn parse(input: &str) -> Option<LanguageMap> {
                if input.is_empty() {
                    return None
                }

                $(
                    #[cfg(feature = $feature)]
                    fn $name($parse_param: &str) -> Result<LanguageMap, Box<Error>> {
                        Ok({ $parse_kode })
                    }

                    // attributes are not yet allowed on `if` expressions
                    #[cfg(feature = $feature)]
                    {
                        if let Ok(result) = $name(input) {
                            return Some(result)
                        }
                    }
                )+

                // Didn't match any of the compiled serialization formats
                None
            }

            pub fn print(&self, _languages: Languages) -> Result<String, Box<Error>> {
                match *self {
                    $(
                        #[cfg(feature = $feature)] Format::$variant => {
                            #[cfg(feature = $feature)]
                            fn print($print_param: Languages) -> Result<String, Box<Error>> {
                                Ok({ $print_kode })
                            }

                            print(_languages)
                        }
                    ),+
                }
            }
        }

        impl FromStr for Format {
            type Err = String;

            fn from_str(format: &str) -> Result<Self, Self::Err> {
                match format {
                    $(
                        stringify!($name) => {
                            #[cfg(feature = $feature)]
                            return Ok(Format::$variant);

                            #[cfg(not(feature = $feature))]
                            return Err(format!(
"This version of tokei was compiled without \
any '{format}' serialization support, to enable serialization, \
reinstall tokei with the features flag.

    cargo install tokei --features {format}

If you want to enable all supported serialization formats, you can use the 'all' feature.

    cargo install tokei --features all\n",
                                format = stringify!($name))
                            );
                        }
                    ),+
                    format => Err(format!("{:?} is not a supported serialization format", format)),
                }
            }
        }
    )
}

// The ordering of these determines the attempted order when parsing.
supported_formats!(
    (cbor, "cbor", Cbor) =>
        |input| {
            extern crate serde_cbor;
            extern crate hex;

            use std::error::Error;
            use std::process;

            let hex: Vec<u8> = match hex::FromHex::from_hex(input) {
                Ok(hex) => hex,
                Err(err) => {
                    eprintln!("{}", err.description());
                    process::exit(1)
                }
            };
            serde_cbor::from_slice(&hex)?
        },
        |languages| {
            let cbor: Vec<u8> = languages.to_cbor()?;

            let mut s = String::new();
            for byte in cbor {
                s.push_str(&format!("{:02x}", byte))
            }
            s
         },

    (json, "json", Json) =>
        |input| {
            extern crate serde_json;
            serde_json::from_str(&input)?
        },
        |languages| {
            languages.to_json()?
        },

    (yaml, "yaml", Yaml) =>
        |input| {
            extern crate serde_yaml;
            serde_yaml::from_str(&input)?
        },
        |languages| {
            languages.to_yaml()?
        },

    (toml, "toml-io", Toml) =>
        |input| {
            extern crate toml;
            toml::from_str(&input)
        },
        |languages| {
            languages.to_toml()?
        },
);

pub fn add_input(input: &str, languages: &mut Languages) {
    use std::fs::File;
    use std::io::Read;
    use std::process;

    let map = match File::open(input) {
        Ok(mut file) => {
            let contents = {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Couldn't read file");
                contents
            };

            convert_input(contents)
        }
        Err(_) => {
            if input == "stdin" {
                let mut stdin = ::std::io::stdin();
                let mut buffer = String::new();

                let _ = stdin.read_to_string(&mut buffer);
                convert_input(buffer)
            } else {
                convert_input(String::from(input))
            }
        }
    };

    if let Some(map) = map {
        *languages += map;
    } else {
        eprintln!("Error:\n Failed to parse input file: {}", input);

        let not_supported = self::Format::not_supported();
        if !not_supported.is_empty() {
            eprintln!("
This version of tokei was compiled without serialization support for the following formats:

    {not_supported}

You may want to install any comma separated combination of {all:?}:

    cargo install tokei --features {all:?}

Or use the 'all' feature:

    cargo install tokei --features all
    \n",
                not_supported = not_supported.join(", "),
                // no space after comma to ease copypaste
                all = self::Format::all().join(",")
            );
        }

        process::exit(1);
    }
}

fn convert_input(contents: String) -> Option<LanguageMap> {
    self::Format::parse(&contents)
}
