use std::convert::From;

use serde_yaml;

pub enum RepoError {
    UnknownKeyAlgorithm(String),
    YamlError(serde_yaml::Error),
}

impl From<serde_yaml::Error> for RepoError {
    fn from(error: serde_yaml::Error) -> Self {
        RepoError::YamlError(error)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
