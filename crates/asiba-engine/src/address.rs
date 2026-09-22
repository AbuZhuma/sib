use std::net::IpAddr;

pub fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local(),
    }
}

pub fn is_public_text(ip: &str) -> bool {
    ip.parse::<IpAddr>().is_ok_and(|ip| !is_private(ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_loopback_and_unspecified_are_not_public() {
        assert!(is_private("127.0.0.1".parse().expect("ip")));
        assert!(is_private("10.1.2.3".parse().expect("ip")));
        assert!(is_private("192.168.0.5".parse().expect("ip")));
        assert!(is_private("::1".parse().expect("ip")));
        assert!(!is_private("8.8.8.8".parse().expect("ip")));
    }

    #[test]
    fn public_text_rejects_private_and_garbage() {
        assert!(!is_public_text("10.0.0.1"));
        assert!(is_public_text("203.0.113.9"));
        assert!(!is_public_text("garbage"));
    }
}
