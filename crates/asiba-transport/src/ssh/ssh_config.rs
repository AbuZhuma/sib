use std::path::PathBuf;

use ssh2_config::{ParseRule, SshConfig};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedHost {
    pub host_name: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub identity_files: Vec<PathBuf>,
}

pub fn resolve(host: &str) -> ResolvedHost {
    let rules = ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS;
    let Ok(config) = SshConfig::parse_default_file(rules) else {
        return ResolvedHost::default();
    };
    from_config(&config, host)
}

fn from_config(config: &SshConfig, host: &str) -> ResolvedHost {
    let params = config.query(host);
    ResolvedHost {
        host_name: params.host_name,
        port: params.port,
        user: params.user,
        identity_files: params
            .identity_file
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_file())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    const SAMPLE: &str = "Host neo\n  HostName 203.0.113.10\n  Port 2222\n  User deploy\n";

    #[test]
    fn alias_resolves_to_host_name_port_and_user() {
        let mut reader = BufReader::new(SAMPLE.as_bytes());
        let config = SshConfig::default()
            .parse(&mut reader, ParseRule::STRICT)
            .expect("config");
        let resolved = from_config(&config, "neo");
        assert_eq!(resolved.host_name.as_deref(), Some("203.0.113.10"));
        assert_eq!(resolved.port, Some(2222));
        assert_eq!(resolved.user.as_deref(), Some("deploy"));
    }

    #[test]
    fn unknown_host_resolves_to_defaults() {
        let mut reader = BufReader::new(SAMPLE.as_bytes());
        let config = SshConfig::default()
            .parse(&mut reader, ParseRule::STRICT)
            .expect("config");
        let resolved = from_config(&config, "other");
        assert_eq!(resolved.host_name, None);
        assert_eq!(resolved.port, None);
    }
}
