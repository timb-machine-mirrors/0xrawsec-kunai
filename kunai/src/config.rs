use core::str::FromStr;
use kunai_common::{
    config::{BpfConfig, Filter, Loader},
    events,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid output {0}")]
    InvalidOutput(String),
    #[error("invalid event {0}")]
    InvalidEvent(String),
}

/// Kunai configuration structure to be used in userland
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub output: String,
    pub max_buffered_events: u16,
    pub events: Vec<Event>,
}

impl Default for Config {
    fn default() -> Self {
        let mut events = vec![];
        for v in events::Type::variants() {
            // some events get disabled by default because there are too many
            let en = !matches!(v, events::Type::Read | events::Type::Write);

            if v.is_configurable() {
                events.push(Event {
                    name: v.as_str().into(),
                    enable: en,
                })
            }
        }

        Self {
            output: "/dev/stdout".into(),
            max_buffered_events: 256,
            events,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        if self.output != "/dev/stdout" {
            return Err(Error::InvalidOutput(self.output.clone()));
        }

        for e in self.events.iter() {
            let Ok(ty) = events::Type::from_str(&e.name) else {
                return Err(Error::InvalidEvent(e.name.clone()));
            };

            if !ty.is_configurable() {
                return Err(Error::InvalidEvent(e.name.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Event {
    name: String,
    enable: bool,
}

impl Config {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub fn from_toml<S: AsRef<str>>(toml: S) -> Result<Self, toml::de::Error> {
        toml::from_str(toml.as_ref())
    }
}

impl TryFrom<Config> for Filter {
    type Error = Error;

    fn try_from(value: Config) -> Result<Self, Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Config> for Filter {
    type Error = Error;

    fn try_from(value: &Config) -> Result<Self, Error> {
        let mut filter = Filter::all_disabled();

        for e in value.events.iter() {
            // config should have been verified so it should not fail
            let ty =
                events::Type::from_str(&e.name).map_err(|_| Error::InvalidEvent(e.name.clone()))?;
            // we enable event in BpfConfig only if it has been configured
            if e.enable {
                filter.enable(ty);
            }
        }

        Ok(filter)
    }
}

impl TryFrom<Config> for BpfConfig {
    type Error = Error;

    fn try_from(value: Config) -> Result<Self, Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Config> for BpfConfig {
    type Error = Error;

    fn try_from(value: &Config) -> Result<Self, Error> {
        Ok(Self {
            loader: Loader::from_own_pid(),
            filter: value.try_into()?,
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_serialize() {
        let config = Config {
            ..Default::default()
        };

        config.validate().unwrap();

        println!("{}", toml::to_string_pretty(&config).unwrap());
    }
}