use std::{
    collections::HashMap,
    fmt::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Default)]
pub struct AppMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Default)]
struct MetricsInner {
    http_requests: Mutex<HashMap<HttpRequestMetric, u64>>,
    password_auth_successes: AtomicU64,
    password_auth_failures: AtomicU64,
    oauth_auth_successes: AtomicU64,
    oauth_auth_failures: AtomicU64,
    refresh_rotations: AtomicU64,
    logouts: AtomicU64,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct HttpRequestMetric {
    method: String,
    path: String,
    status: u16,
}

impl AppMetrics {
    pub fn record_http_request(&self, method: &str, path: &str, status: u16) {
        let key = HttpRequestMetric {
            method: method.to_string(),
            path: path.to_string(),
            status,
        };

        let mut requests = self.inner.http_requests.lock().unwrap();
        *requests.entry(key).or_insert(0) += 1;
    }

    pub fn record_password_auth_success(&self) {
        self.inner
            .password_auth_successes
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_password_auth_failure(&self) {
        self.inner
            .password_auth_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_oauth_auth_success(&self) {
        self.inner
            .oauth_auth_successes
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_oauth_auth_failure(&self) {
        self.inner
            .oauth_auth_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_refresh_rotation(&self) {
        self.inner.refresh_rotations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_logout(&self) {
        self.inner.logouts.fetch_add(1, Ordering::Relaxed);
    }

    pub fn render_prometheus(&self) -> String {
        let mut output = String::new();

        output.push_str("# TYPE authwarden_http_requests_total counter\n");
        let mut requests = self
            .inner
            .http_requests
            .lock()
            .unwrap()
            .iter()
            .map(|(key, count)| (key.clone(), *count))
            .collect::<Vec<_>>();
        requests.sort_by(|(left, _), (right, _)| {
            left.method
                .cmp(&right.method)
                .then(left.path.cmp(&right.path))
                .then(left.status.cmp(&right.status))
        });

        for (key, count) in requests {
            writeln!(
                output,
                "authwarden_http_requests_total{{method=\"{}\",path=\"{}\",status=\"{}\"}} {}",
                escape_label_value(&key.method),
                escape_label_value(&key.path),
                key.status,
                count
            )
            .unwrap();
        }

        writeln!(output, "# TYPE authwarden_auth_success_total counter").unwrap();
        write_labeled_sample(
            &mut output,
            "authwarden_auth_success_total",
            "flow",
            "password",
            self.inner.password_auth_successes.load(Ordering::Relaxed),
        );
        write_labeled_sample(
            &mut output,
            "authwarden_auth_success_total",
            "flow",
            "oauth",
            self.inner.oauth_auth_successes.load(Ordering::Relaxed),
        );

        writeln!(output, "# TYPE authwarden_auth_failure_total counter").unwrap();
        write_labeled_sample(
            &mut output,
            "authwarden_auth_failure_total",
            "flow",
            "password",
            self.inner.password_auth_failures.load(Ordering::Relaxed),
        );
        write_labeled_sample(
            &mut output,
            "authwarden_auth_failure_total",
            "flow",
            "oauth",
            self.inner.oauth_auth_failures.load(Ordering::Relaxed),
        );
        write_counter_without_label(
            &mut output,
            "authwarden_refresh_rotations_total",
            self.inner.refresh_rotations.load(Ordering::Relaxed),
        );
        write_counter_without_label(
            &mut output,
            "authwarden_logouts_total",
            self.inner.logouts.load(Ordering::Relaxed),
        );

        output
    }
}

fn write_labeled_sample(output: &mut String, name: &str, label: &str, value: &str, count: u64) {
    writeln!(
        output,
        "{name}{{{label}=\"{}\"}} {count}",
        escape_label_value(value)
    )
    .unwrap();
}

fn write_counter_without_label(output: &mut String, name: &str, count: u64) {
    writeln!(output, "# TYPE {name} counter").unwrap();
    writeln!(output, "{name} {count}").unwrap();
}

fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_http_and_auth_counters() {
        let metrics = AppMetrics::default();
        metrics.record_http_request("GET", "/health", 200);
        metrics.record_password_auth_success();
        metrics.record_password_auth_failure();
        metrics.record_refresh_rotation();
        metrics.record_logout();

        let output = metrics.render_prometheus();

        assert!(output.contains(
            "authwarden_http_requests_total{method=\"GET\",path=\"/health\",status=\"200\"} 1"
        ));
        assert!(output.contains("authwarden_auth_success_total{flow=\"password\"} 1"));
        assert!(output.contains("authwarden_auth_failure_total{flow=\"password\"} 1"));
        assert!(output.contains("authwarden_refresh_rotations_total 1"));
        assert!(output.contains("authwarden_logouts_total 1"));
    }
}
