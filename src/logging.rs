use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredError {
    pub operation: String,
    pub path: Option<PathBuf>,
    pub kind: String,
    pub message: String,
}

impl StructuredError {
    pub fn new(
        operation: impl Into<String>,
        path: Option<PathBuf>,
        kind: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            path,
            kind: kind.into(),
            message: message.into(),
        }
    }

    pub fn from_io(
        operation: impl Into<String>,
        path: impl Into<PathBuf>,
        err: std::io::Error,
    ) -> Self {
        let operation = operation.into();
        let message = format!("{operation} failed: {err}");
        Self {
            operation,
            path: Some(path.into()),
            kind: format!("{:?}", err.kind()),
            message,
        }
    }

    pub fn from_json(
        operation: impl Into<String>,
        path: Option<PathBuf>,
        err: serde_json::Error,
    ) -> Self {
        let operation = operation.into();
        let message = format!("{operation} failed: {err}");
        Self {
            operation,
            path,
            kind: "json".to_string(),
            message,
        }
    }
}

pub fn warn(message: &str, error: Option<&StructuredError>) {
    match error {
        Some(details) => {
            eprintln!(
                "[warn] {message} | op={} kind={} path={:?} msg={}",
                details.operation, details.kind, details.path, details.message
            );
        }
        None => eprintln!("[warn] {message}"),
    }
}

pub fn error(message: &str, error: &StructuredError) {
    eprintln!(
        "[error] {message} | op={} kind={} path={:?} msg={}",
        error.operation, error.kind, error.path, error.message
    );
}
