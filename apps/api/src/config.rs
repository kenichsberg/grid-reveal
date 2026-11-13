//! Startup configuration, read once from the environment.
//!
//! Every required variable is validated here so the process fails at boot with
//! a precise message rather than at the first request that needs it.

use std::env::{self, VarError};
use std::fmt;

#[derive(Debug)]
pub struct ConfigError {
    pub variable: &'static str,
    pub reason: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "configuration error: {} {}\n\
             Copy .env.example to .env and fill it in.",
            self.variable, self.reason
        )
    }
}

impl std::error::Error for ConfigError {}

fn required(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        Ok(_) => Err(ConfigError {
            variable: name,
            reason: "is set but empty; it must have a value".into(),
        }),
        Err(VarError::NotPresent) => Err(ConfigError {
            variable: name,
            reason: "is required but not set".into(),
        }),
        Err(VarError::NotUnicode(_)) => Err(ConfigError {
            variable: name,
            reason: "contains invalid UTF-8".into(),
        }),
    }
}

fn optional(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn parsed<T: std::str::FromStr>(name: &'static str, default: T) -> Result<T, ConfigError> {
    match optional(name) {
        None => Ok(default),
        Some(raw) => raw.parse::<T>().map_err(|_| ConfigError {
            variable: name,
            reason: format!("could not be parsed from {raw:?}"),
        }),
    }
}

/// Where processed pictures are stored.
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// S3-compatible object storage. Cloudflare R2 in production.
    S3 {
        endpoint: String,
        bucket: String,
        access_key_id: String,
        secret_access_key: String,
        /// Public base URL if the bucket is served from a domain. When absent
        /// the API streams picture bytes itself.
        public_base_url: Option<String>,
    },
    /// Local filesystem. Development only; a single VM's disk is not durable
    /// storage for anything that matters.
    Local { dir: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,

    /// Domain the session cookie is scoped to, covering both the web and API
    /// hosts. `None` yields a host-only cookie, which only works locally.
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
    pub session_ttl_days: i64,

    pub cors_allowed_origins: Vec<String>,

    pub admin_username: String,
    pub admin_password: String,

    pub storage: StorageConfig,
}

impl Config {
    /// Reads and validates configuration, loading `.env` first if present.
    pub fn from_env() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let database_url = required("DATABASE_URL")?;
        let admin_username = required("ADMIN_USERNAME")?;
        let admin_password = required("ADMIN_PASSWORD")?;

        if admin_password.chars().count() < super::auth::password::MIN_PASSWORD_LEN {
            return Err(ConfigError {
                variable: "ADMIN_PASSWORD",
                reason: format!(
                    "must be at least {} characters",
                    super::auth::password::MIN_PASSWORD_LEN
                ),
            });
        }

        let cookie_secure: bool = parsed("COOKIE_SECURE", true)?;
        let session_ttl_days: i64 = parsed("SESSION_TTL_DAYS", 30)?;
        if session_ttl_days < 1 {
            return Err(ConfigError {
                variable: "SESSION_TTL_DAYS",
                reason: "must be at least 1".into(),
            });
        }

        let cors_allowed_origins = optional("CORS_ALLOWED_ORIGINS")
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let storage = Self::storage_from_env()?;

        Ok(Config {
            database_url,
            bind_addr: optional("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            cookie_domain: optional("COOKIE_DOMAIN"),
            cookie_secure,
            session_ttl_days,
            cors_allowed_origins,
            admin_username,
            admin_password,
            storage,
        })
    }

    /// S3 is used when an endpoint is configured; otherwise local disk, which
    /// keeps development running without object storage credentials.
    fn storage_from_env() -> Result<StorageConfig, ConfigError> {
        match optional("R2_ENDPOINT") {
            Some(endpoint) => Ok(StorageConfig::S3 {
                endpoint,
                bucket: required("R2_BUCKET")?,
                access_key_id: required("R2_ACCESS_KEY_ID")?,
                secret_access_key: required("R2_SECRET_ACCESS_KEY")?,
                public_base_url: optional("R2_PUBLIC_BASE_URL"),
            }),
            None => Ok(StorageConfig::Local {
                dir: optional("LOCAL_STORAGE_DIR").unwrap_or_else(|| "./data/pictures".into()),
            }),
        }
    }
}
