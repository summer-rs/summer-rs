use std::io::{self, ErrorKind};
use thiserror::Error;

/// summer custom error type
#[derive(Error, Debug)]
pub enum AppError {
    /// component not exists
    #[error("{0} component not exists")]
    ComponentNotExist(&'static str),

    /// `.env` file reading failed
    #[error(transparent)]
    EnvError(#[from] dotenvy::Error),

    /// File IO Error
    #[error(transparent)]
    IOError(#[from] io::Error),

    /// toml file parsing error
    #[error(transparent)]
    TomlParseError(#[from] toml::de::Error),

    /// Configuration merge error in toml file
    #[error("merge toml error: {0}")]
    TomlMergeError(String),

    /// tokio asynchronous task join failed
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    /// Deserialization of configuration in toml file to rust struct failed
    #[error("Failed to deserialize the configuration of prefix \"{0}\": {1}")]
    DeserializeErr(&'static str, toml::de::Error),

    /// Other runtime errors
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

impl AppError {
    /// Failed to read file io
    pub fn from_io(kind: ErrorKind, msg: &str) -> Self {
        AppError::IOError(io::Error::new(kind, msg))
    }
}

/// Contains the return value of AppError
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_not_exist_error() {
        let error = AppError::ComponentNotExist("TestComponent");
        let error_msg = error.to_string();
        assert!(error_msg.contains("TestComponent"));
        assert!(error_msg.contains("component not exists"));
    }

    #[test]
    fn test_from_io_error() {
        let error = AppError::from_io(ErrorKind::NotFound, "file not found");
        match error {
            AppError::IOError(e) => {
                assert_eq!(e.kind(), ErrorKind::NotFound);
                assert!(e.to_string().contains("file not found"));
            }
            _ => panic!("Expected IOError"),
        }
    }

    #[test]
    fn test_toml_merge_error() {
        let error = AppError::TomlMergeError("conflicting keys".to_string());
        let error_msg = error.to_string();
        assert!(error_msg.contains("merge toml error"));
        assert!(error_msg.contains("conflicting keys"));
    }

    #[test]
    fn test_deserialize_error() {
        let toml_err = toml::from_str::<i32>("invalid").unwrap_err();
        let error = AppError::DeserializeErr("my-config", toml_err);
        let error_msg = error.to_string();
        assert!(error_msg.contains("Failed to deserialize"));
        assert!(error_msg.contains("my-config"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(ErrorKind::PermissionDenied, "access denied");
        let app_error: AppError = io_error.into();
        
        match app_error {
            AppError::IOError(e) => {
                assert_eq!(e.kind(), ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected IOError"),
        }
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let app_error: AppError = anyhow_err.into();
        
        match app_error {
            AppError::OtherError(e) => {
                assert!(e.to_string().contains("something went wrong"));
            }
            _ => panic!("Expected OtherError"),
        }
    }

    #[test]
    fn test_error_result_type() {
        fn returns_error() -> Result<i32> {
            Err(AppError::ComponentNotExist("TestComponent"))
        }

        let result = returns_error();
        assert!(result.is_err());
        
        match result {
            Err(AppError::ComponentNotExist(name)) => {
                assert_eq!(name, "TestComponent");
            }
            _ => panic!("Expected ComponentNotExist error"),
        }
    }

    #[test]
    fn test_error_result_ok() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        let result = returns_ok();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_error_chain() {
        fn nested_error() -> Result<()> {
            std::fs::read_to_string("/nonexistent/file.txt")?;
            Ok(())
        }

        let result = nested_error();
        assert!(result.is_err());
        
        match result {
            Err(AppError::IOError(_)) => {
                // Expected
            }
            _ => panic!("Expected IOError from file operation"),
        }
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors = vec![
            AppError::ComponentNotExist("Test"),
            AppError::from_io(ErrorKind::NotFound, "test"),
            AppError::TomlMergeError("test merge".to_string()),
        ];

        for error in errors {
            // All errors should have meaningful display messages
            let msg = error.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }
}
