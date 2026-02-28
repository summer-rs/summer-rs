use crate::error::{AppError, Result};
use anyhow::Context;
use std::{
    env,
    ffi::OsStr,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// App environment
#[derive(Debug, Clone, Copy, Default)]
pub enum Env {
    /// Development
    #[default]
    Dev,
    /// Test
    Test,
    /// production
    Prod,
}

impl Env {
    /// Initializes environment variables from the `.env` file and reads `summer_ENV` to determine the active environment for the application.
    pub fn init() -> Self {
        match dotenvy::dotenv() {
            Ok(path) => {
                log::debug!("Loaded the environment variable file under the path: \"{path:?}\"",)
            }
            Err(e) => log::debug!("Environment variable file not found: {e}"),
        }

        Self::from_env()
    }

    /// Read `summer_ENV` to determine the environment of the active application.
    /// If there is no `summer_ENV` variable, it defaults to Dev
    pub fn from_env() -> Self {
        match env::var("summer_ENV") {
            Ok(var) => Self::from_string(var),
            Err(_) => Self::Dev,
        }
    }

    /// Parse the string to get the corresponding environment
    pub fn from_string<S: Into<String>>(str: S) -> Self {
        match str.into() {
            s if s.eq_ignore_ascii_case("dev") => Self::Dev,
            s if s.eq_ignore_ascii_case("test") => Self::Test,
            s if s.eq_ignore_ascii_case("prod") => Self::Prod,
            _ => Self::Dev,
        }
    }

    pub(crate) fn get_config_path(&self, path: &Path) -> Result<PathBuf> {
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("");
        let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
        let canonicalize = path
            .canonicalize()
            .with_context(|| format!("canonicalize {path:?} failed"))?;
        let parent = canonicalize
            .parent()
            .ok_or_else(|| AppError::from_io(ErrorKind::NotFound, "config file path not found"))?;
        Ok(match self {
            Self::Dev => parent.join(format!("{stem}-dev.{ext}")),
            Self::Test => parent.join(format!("{stem}-test.{ext}")),
            Self::Prod => parent.join(format!("{stem}-prod.{ext}")),
        })
    }
}

pub(crate) fn interpolate(template: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = template.chars().collect();

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            // find "}"
            let mut j = i + 2; // Skip `${`
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }

            if j < chars.len() && chars[j] == '}' {
                // extract var_name & default_value
                let placeholder: String = chars[i + 2..j].iter().collect();

                // find default_value
                if let Some(pos) = placeholder.find(':') {
                    let var_name = &placeholder[..pos];
                    if let Ok(value) = env::var(var_name) {
                        result.push_str(&value);
                    } else {
                        result.push_str(&placeholder[pos + 1..]);
                    }
                } else if let Ok(value) = env::var(&placeholder) {
                    result.push_str(&value);
                } else {
                    result.push_str("${");
                    result.push_str(&placeholder);
                    result.push('}');
                }

                i = j + 1; // move to next
            } else {
                result.push('$');
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::Env;
    use crate::error::Result;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_get_config_path() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        let temp_dir = temp_dir.path().canonicalize()?;

        let foo = temp_dir.join("foo.toml");
        let _ = touch(&foo);

        assert_eq!(
            Env::from_string("dev").get_config_path(foo.as_path())?,
            temp_dir.join("foo-dev.toml")
        );

        assert_eq!(
            Env::from_string("test").get_config_path(foo.as_path())?,
            temp_dir.join("foo-test.toml")
        );

        assert_eq!(
            Env::from_string("prod").get_config_path(foo.as_path())?,
            temp_dir.join("foo-prod.toml")
        );

        assert_eq!(
            Env::from_string("other").get_config_path(foo.as_path())?,
            temp_dir.join("foo-dev.toml")
        );

        Ok(())
    }

    #[test]
    fn test_env() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir.path().canonicalize()?;
        let foo = temp_dir.join("foo.toml");
        let _ = touch(&foo);

        std::env::set_var("summer_ENV", "dev");
        assert_eq!(
            Env::from_env().get_config_path(foo.as_path())?,
            temp_dir.join("foo-dev.toml")
        );

        std::env::set_var("summer_ENV", "TEST");
        assert_eq!(
            Env::from_env().get_config_path(foo.as_path())?,
            temp_dir.join("foo-test.toml")
        );

        std::env::set_var("summer_ENV", "Prod");
        assert_eq!(
            Env::from_env().get_config_path(foo.as_path())?,
            temp_dir.join("foo-prod.toml")
        );

        std::env::set_var("summer_ENV", "Other");
        assert_eq!(
            Env::from_env().get_config_path(foo.as_path())?,
            temp_dir.join("foo-dev.toml")
        );

        Ok(())
    }

    #[allow(dead_code)]
    fn touch(path: &PathBuf) -> Result<()> {
        let _ = fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(path)?;
        Ok(())
    }

    #[test]
    fn test_interpolate() {
        std::env::set_var("NAME", "Alice");

        let template = "Hello, ${NAME:default_name}!";
        let result = super::interpolate(template);
        assert_eq!("Hello, Alice!", result);

        std::env::remove_var("NAME");
        let result = super::interpolate(template);
        assert_eq!("Hello, default_name!", result);

        let template = "Hello, ${UNKNOWN_NAME}!";
        let result = super::interpolate(template);
        assert_eq!("Hello, ${UNKNOWN_NAME}!", result);

        let template = "你好, ${UNKNOWN_NAME:默认值}!";
        let result = super::interpolate(template);
        assert_eq!("你好, 默认值!", result);
    }
}
