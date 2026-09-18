use axum::http::HeaderMap;

pub fn client_identifier(
    headers: &HeaderMap,
    trust_proxy_headers: bool,
    direct_client_ip: Option<&str>,
) -> String {
    if trust_proxy_headers && let Some(forwarded_client_ip) = forwarded_client_identifier(headers) {
        return forwarded_client_ip.to_string();
    }

    direct_client_ip.unwrap_or("unknown").to_string()
}

fn forwarded_client_identifier(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn extracts_first_forwarded_client_ip_when_proxy_headers_are_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.0.0.1"),
        );

        assert_eq!(
            client_identifier(&headers, true, Some("198.51.100.10")),
            "203.0.113.10"
        );
    }

    #[test]
    fn ignores_forwarded_client_ip_when_proxy_headers_are_not_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.10"));

        assert_eq!(
            client_identifier(&headers, false, Some("198.51.100.10")),
            "198.51.100.10"
        );
    }

    #[test]
    fn falls_back_to_real_ip_when_proxy_headers_are_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("203.0.113.20"));

        assert_eq!(
            client_identifier(&headers, true, Some("198.51.100.10")),
            "203.0.113.20"
        );
    }

    #[test]
    fn falls_back_to_unknown_without_socket_ip() {
        assert_eq!(client_identifier(&HeaderMap::new(), false, None), "unknown");
    }
}
