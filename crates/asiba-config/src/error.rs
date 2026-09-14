use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("не удалось определить домашнюю директорию")]
    NoHome,
    #[error("ошибка чтения {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("ошибка записи {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("ошибка разбора {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("ошибка сериализации: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("хранилище секретов: {0}")]
    Secrets(String),
}
