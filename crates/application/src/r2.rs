//! Private Cloudflare R2 adapter behind [`ObjectStorage`].
//!
//! R2 speaks the S3 API and authenticates every request with AWS Signature
//! Version 4, so this adapter signs plain HTTP requests itself: one `PUT`,
//! `GET` or `DELETE` per call, path-style addressing
//! (`{endpoint}/{bucket}/{key}`), and no public URLs. The signing is checked
//! against AWS's published example, and the request flow against a local mock
//! server, so the adapter is verified before a real bucket exists.
//!
//! Presigned URLs use the same signature over the query string instead of a
//! header. They let a client send or fetch bytes directly for a short time
//! without the API relaying them.

use std::time::Duration;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::storage::{ObjectStorage, StorageError, StorageFuture};

type HmacSha256 = Hmac<Sha256>;

/// R2 treats the region as a fixed placeholder.
pub const R2_REGION: &str = "auto";
const SERVICE: &str = "s3";
/// Longest presigned lifetime S3 allows; callers should use far less.
const MAX_PRESIGN_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Credentials and target for one bucket. Values come from the environment or
/// a secret manager; they are never logged.
#[derive(Clone)]
pub struct R2Config {
    /// `https://<account-id>.r2.cloudflarestorage.com` (or a test server).
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
}

impl std::fmt::Debug for R2Config {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("R2Config")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field("region", &self.region)
            .finish()
    }
}

pub struct R2Storage {
    config: R2Config,
    http: reqwest::Client,
}

impl R2Storage {
    /// Reads `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID` and
    /// `R2_SECRET_ACCESS_KEY`. `Ok(None)` when none are set; an error when only
    /// some are, so a half-configured bucket never falls back silently.
    pub fn from_env() -> Result<Option<Self>, StorageError> {
        let read = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let values = [
            read("R2_ENDPOINT"),
            read("R2_BUCKET"),
            read("R2_ACCESS_KEY_ID"),
            read("R2_SECRET_ACCESS_KEY"),
        ];
        if values.iter().all(Option::is_none) {
            return Ok(None);
        }
        let [
            Some(endpoint),
            Some(bucket),
            Some(access_key_id),
            Some(secret_access_key),
        ] = values
        else {
            return Err(StorageError::Unavailable(
                "R2_ENDPOINT, R2_BUCKET, R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY must be set together"
                    .into(),
            ));
        };
        Self::new(R2Config {
            endpoint,
            bucket,
            access_key_id,
            secret_access_key,
            region: R2_REGION.into(),
        })
        .map(Some)
    }

    pub fn new(config: R2Config) -> Result<Self, StorageError> {
        let endpoint = config.endpoint.trim_end_matches('/');
        validate_endpoint(endpoint)?;
        // Normalize once, here: `reqwest` lower-cases the host and elides a
        // default port when it builds the request line and `Host` header, so
        // signing the raw authority would cover a different host than the one
        // that reaches the wire.
        let endpoint = normalize_endpoint(endpoint);
        if config.bucket.trim().is_empty()
            || config.access_key_id.trim().is_empty()
            || config.secret_access_key.trim().is_empty()
        {
            return Err(StorageError::Unavailable(
                "R2 bucket and credentials are required".into(),
            ));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| StorageError::Unavailable(error.to_string()))?;
        Ok(Self {
            config: R2Config { endpoint, ..config },
            http,
        })
    }

    fn host(&self) -> &str {
        self.config
            .endpoint
            .split_once("://")
            .map_or(self.config.endpoint.as_str(), |(_, rest)| rest)
    }

    fn object_path(&self, key: &str) -> Result<String, StorageError> {
        validate_key(key)?;
        Ok(format!(
            "/{}/{}",
            uri_encode(&self.config.bucket, false),
            uri_encode(key, false)
        ))
    }

    async fn send(
        &self,
        method: reqwest::Method,
        key: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<reqwest::Response, StorageError> {
        let path = self.object_path(key)?;
        let now = Utc::now();
        let payload_hash = hex::encode(Sha256::digest(&body));
        let mut headers = vec![
            ("host".to_owned(), self.host().to_owned()),
            ("x-amz-content-sha256".to_owned(), payload_hash.clone()),
            ("x-amz-date".to_owned(), amz_date(now)),
        ];
        if let Some(content_type) = content_type {
            headers.push(("content-type".to_owned(), content_type.to_owned()));
        }
        let authorization = authorization_header(
            &SigningInput {
                method: method.as_str(),
                path: &path,
                query: &[],
                headers: &headers,
                payload_hash: &payload_hash,
            },
            &self.config,
            now,
        );

        let mut request = self
            .http
            .request(method, format!("{}{path}", self.config.endpoint))
            .header("authorization", authorization)
            .body(body);
        for (name, value) in &headers {
            if name != "host" {
                request = request.header(name, value);
            }
        }
        request
            .send()
            .await
            .map_err(|error| StorageError::Failed(format!("R2 request failed: {error}")))
    }

    /// URL a client may use to `PUT` (upload) or `GET` (download) one object
    /// directly, valid for `expires`. The signature pins method, key and host.
    pub fn presigned_url(
        &self,
        method: &str,
        key: &str,
        expires: Duration,
        now: DateTime<Utc>,
    ) -> Result<String, StorageError> {
        if !matches!(method, "GET" | "PUT") {
            return Err(StorageError::Failed(
                "only GET and PUT can be presigned".into(),
            ));
        }
        let seconds = expires.as_secs();
        if seconds == 0 || seconds > MAX_PRESIGN_SECONDS {
            return Err(StorageError::Failed(
                "presigned lifetime is out of range".into(),
            ));
        }
        let path = self.object_path(key)?;
        Ok(presign(
            &self.config,
            self.host(),
            method,
            &path,
            seconds,
            now,
        ))
    }
}

impl ObjectStorage for R2Storage {
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
    ) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let response = self
                .send(reqwest::Method::PUT, key, bytes, Some(content_type))
                .await?;
            ensure_success(response).await.map(|_| ())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> StorageFuture<'a, Result<Option<Vec<u8>>, StorageError>> {
        Box::pin(async move {
            let response = self
                .send(reqwest::Method::GET, key, Vec::new(), None)
                .await?;
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }
            let response = ensure_success(response).await?;
            let bytes = response
                .bytes()
                .await
                .map_err(|error| StorageError::Failed(format!("R2 read failed: {error}")))?;
            Ok(Some(bytes.to_vec()))
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let response = self
                .send(reqwest::Method::DELETE, key, Vec::new(), None)
                .await?;
            // S3 answers 204 for a delete, including one of a missing key.
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(());
            }
            ensure_success(response).await.map(|_| ())
        })
    }
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, StorageError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    // The body names the S3 error code; it never carries credentials.
    let body = response.text().await.unwrap_or_default();
    let code = body
        .split("<Code>")
        .nth(1)
        .and_then(|rest| rest.split("</Code>").next())
        .unwrap_or("unknown");
    Err(StorageError::Failed(format!(
        "R2 returned {status} ({code})"
    )))
}

/// Keys come from the server (`tenants/{tenant}/attachments/{id}`), but the
/// adapter still refuses anything that could change the request path.
fn validate_key(key: &str) -> Result<(), StorageError> {
    let valid = !key.is_empty()
        && key.len() <= 1024
        && !key.starts_with('/')
        && key
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'));
    if valid {
        Ok(())
    } else {
        Err(StorageError::Failed("invalid object key".into()))
    }
}

/// The endpoint is exactly `scheme://host[:port]`. A path, query, fragment or
/// credentials would end up in the signed `Host` or request line, so the
/// adapter would start but every request would fail its signature; refuse at
/// start-up instead. Plain http is allowed only for a loopback test server.
fn validate_endpoint(endpoint: &str) -> Result<(), StorageError> {
    let refuse = |why: &str| {
        Err(StorageError::Unavailable(format!(
            "R2_ENDPOINT must be https://<account-id>.r2.cloudflarestorage.com ({why})"
        )))
    };
    let Some((scheme, authority)) = endpoint.split_once("://") else {
        return refuse("missing scheme");
    };
    if authority.is_empty() {
        return refuse("missing host");
    }
    if authority.contains(['/', '?', '#', '@', ' ']) {
        return refuse("no path, query, fragment or credentials");
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port
        && (port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()))
    {
        return refuse("invalid port");
    }
    let host_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'));
    if !host_ok {
        return refuse("invalid host");
    }
    match scheme {
        "https" => Ok(()),
        "http" if host == "127.0.0.1" => Ok(()),
        _ => refuse("https only; http is for a loopback test server"),
    }
}

/// Lower-cases the scheme and host and drops a port that is the scheme's
/// default. The result is the single authority used both for signing and for
/// the request URL, so a signature can never cover a different host than the
/// one `reqwest` puts on the wire.
fn normalize_endpoint(endpoint: &str) -> String {
    let Some((scheme, authority)) = endpoint.split_once("://") else {
        return endpoint.to_owned();
    };
    let scheme = scheme.to_ascii_lowercase();
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let host = host.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "https" => Some("443"),
        "http" => Some("80"),
        _ => None,
    };
    match port {
        Some(port) if Some(port) != default_port => format!("{scheme}://{host}:{port}"),
        _ => format!("{scheme}://{host}"),
    }
}

fn amz_date(now: DateTime<Utc>) -> String {
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

/// RFC 3986 encoding as SigV4 requires. `/` stays literal in paths only.
fn uri_encode(value: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

struct SigningInput<'a> {
    method: &'a str,
    path: &'a str,
    query: &'a [(String, String)],
    /// Lower-case header names with trimmed values.
    headers: &'a [(String, String)],
    payload_hash: &'a str,
}

fn canonical_query(query: &[(String, String)]) -> String {
    let mut pairs: Vec<(String, String)> = query
        .iter()
        .map(|(name, value)| (uri_encode(name, true), uri_encode(value, true)))
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn canonical_request(input: &SigningInput<'_>) -> (String, String) {
    let mut headers: Vec<(String, String)> = input
        .headers
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    headers.sort();
    let canonical_headers: String = headers
        .iter()
        .map(|(name, value)| format!("{name}:{value}\n"))
        .collect();
    let signed_headers = headers
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let request = format!(
        "{}\n{}\n{}\n{canonical_headers}\n{signed_headers}\n{}",
        input.method,
        input.path,
        canonical_query(input.query),
        input.payload_hash
    );
    (request, signed_headers)
}

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn signature(config: &R2Config, now: DateTime<Utc>, canonical: &str) -> (String, String) {
    let date = now.format("%Y%m%d").to_string();
    let scope = format!("{date}/{}/{SERVICE}/aws4_request", config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{scope}\n{}",
        amz_date(now),
        hex::encode(Sha256::digest(canonical.as_bytes()))
    );
    let key = hmac(
        &hmac(
            &hmac(
                &hmac(
                    format!("AWS4{}", config.secret_access_key).as_bytes(),
                    &date,
                ),
                &config.region,
            ),
            SERVICE,
        ),
        "aws4_request",
    );
    (hex::encode(hmac(&key, &string_to_sign)), scope)
}

fn authorization_header(input: &SigningInput<'_>, config: &R2Config, now: DateTime<Utc>) -> String {
    let (canonical, signed_headers) = canonical_request(input);
    let (signature, scope) = signature(config, now, &canonical);
    format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        config.access_key_id
    )
}

/// Rebuilds the SigV4 `Authorization` header for one header-signed request
/// from the values the request actually carries.
///
/// This is the same routine [`R2Storage`] uses to sign, exposed so a loopback
/// test server can recompute the expected signature and reject a request whose
/// signed host, path or headers do not match what it received. Production code
/// signs through the private `authorization_header`.
#[doc(hidden)]
pub fn authorization_header_for_test(
    config: &R2Config,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    payload_hash: &str,
    now: DateTime<Utc>,
) -> String {
    authorization_header(
        &SigningInput {
            method,
            path,
            query: &[],
            headers,
            payload_hash,
        },
        config,
        now,
    )
}

fn presign(
    config: &R2Config,
    host: &str,
    method: &str,
    path: &str,
    expires_seconds: u64,
    now: DateTime<Utc>,
) -> String {
    let date = now.format("%Y%m%d").to_string();
    let scope = format!("{date}/{}/{SERVICE}/aws4_request", config.region);
    let query = vec![
        ("X-Amz-Algorithm".to_owned(), "AWS4-HMAC-SHA256".to_owned()),
        (
            "X-Amz-Credential".to_owned(),
            format!("{}/{scope}", config.access_key_id),
        ),
        ("X-Amz-Date".to_owned(), amz_date(now)),
        ("X-Amz-Expires".to_owned(), expires_seconds.to_string()),
        ("X-Amz-SignedHeaders".to_owned(), "host".to_owned()),
    ];
    let headers = [("host".to_owned(), host.to_owned())];
    let (canonical, _) = canonical_request(&SigningInput {
        method,
        path,
        query: &query,
        headers: &headers,
        payload_hash: "UNSIGNED-PAYLOAD",
    });
    let (signature, _) = signature(config, now, &canonical);
    // The signature covers every other parameter, so it is appended last.
    format!(
        "{}{path}?{}&X-Amz-Signature={signature}",
        config.endpoint,
        canonical_query(&query)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Credentials from AWS's public Signature Version 4 examples; not a secret.
    fn aws_example_config(region: &str) -> R2Config {
        R2Config {
            endpoint: "https://examplebucket.s3.amazonaws.com".into(),
            bucket: "examplebucket".into(),
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            region: region.into(),
        }
    }

    fn aws_example_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2013, 5, 24, 0, 0, 0).unwrap()
    }

    /// AWS S3 docs, "Example: GET Object" (header-based SigV4).
    #[test]
    fn header_signature_matches_the_aws_get_object_example() {
        let config = aws_example_config("us-east-1");
        let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let headers = vec![
            (
                "host".to_owned(),
                "examplebucket.s3.amazonaws.com".to_owned(),
            ),
            ("range".to_owned(), "bytes=0-9".to_owned()),
            ("x-amz-content-sha256".to_owned(), empty.to_owned()),
            ("x-amz-date".to_owned(), "20130524T000000Z".to_owned()),
        ];
        let header = authorization_header(
            &SigningInput {
                method: "GET",
                path: "/test.txt",
                query: &[],
                headers: &headers,
                payload_hash: empty,
            },
            &config,
            aws_example_time(),
        );
        assert_eq!(
            header,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    /// AWS S3 docs, "Example: Presigned URL" (query-string SigV4).
    #[test]
    fn presigned_url_matches_the_aws_example() {
        let config = aws_example_config("us-east-1");
        let url = presign(
            &config,
            "examplebucket.s3.amazonaws.com",
            "GET",
            "/test.txt",
            86_400,
            aws_example_time(),
        );
        assert!(
            url.ends_with(
                "X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
            ),
            "{url}"
        );
        assert!(url.contains("X-Amz-Expires=86400"));
        assert!(
            !url.contains("wJalrXUtnFEMI"),
            "the secret must never appear in a URL"
        );
    }

    #[test]
    fn keys_that_could_change_the_path_are_refused() {
        for key in [
            "", "/a", "a//b", "a/../b", "a/./b", "a b", "a?b", "a%2Fb", "a#b",
        ] {
            assert!(validate_key(key).is_err(), "{key:?}");
        }
        assert!(validate_key("tenants/0190-aa/attachments/0190-bb").is_ok());
    }

    #[test]
    fn endpoints_are_bare_hosts() {
        for good in [
            "https://abc123.r2.cloudflarestorage.com",
            "https://abc123.r2.cloudflarestorage.com:443",
            "http://127.0.0.1:9000",
        ] {
            assert!(validate_endpoint(good).is_ok(), "{good}");
        }
        for bad in [
            "abc123.r2.cloudflarestorage.com",
            "https://",
            "https://abc123.r2.cloudflarestorage.com/bucket",
            "https://abc123.r2.cloudflarestorage.com?x=1",
            "https://abc123.r2.cloudflarestorage.com#x",
            "https://user:pass@abc123.r2.cloudflarestorage.com",
            "https://abc123.r2.cloudflarestorage.com:",
            "https://abc123.r2.cloudflarestorage.com:44x",
            "http://abc123.r2.cloudflarestorage.com",
            "http://localhost:9000",
            "ftp://abc123.r2.cloudflarestorage.com",
        ] {
            assert!(validate_endpoint(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn endpoint_normalization_lowercases_hosts_and_drops_default_ports() {
        assert_eq!(
            normalize_endpoint("HTTPS://Bucket.Example.com"),
            "https://bucket.example.com"
        );
        assert_eq!(
            normalize_endpoint("https://Bucket.Example.com:443"),
            "https://bucket.example.com"
        );
        assert_eq!(
            normalize_endpoint("https://Bucket.Example.com:8443"),
            "https://bucket.example.com:8443"
        );
        assert_eq!(
            normalize_endpoint("http://127.0.0.1:9000"),
            "http://127.0.0.1:9000"
        );
        assert_eq!(
            normalize_endpoint("http://127.0.0.1:80"),
            "http://127.0.0.1"
        );
    }

    #[test]
    fn storage_signs_and_requests_the_same_normalized_endpoint() {
        let mut config = aws_example_config(R2_REGION);
        config.endpoint = "https://Bucket.Example.com:443".into();
        let storage = R2Storage::new(config).unwrap();
        assert_eq!(storage.config.endpoint, "https://bucket.example.com");
        assert_eq!(storage.host(), "bucket.example.com");

        // The URL handed to clients is built from `config.endpoint` while the
        // signature is computed over `host()`; both must be the normalized
        // value, so this equals a URL signed directly for the normalized host.
        let now = aws_example_time();
        let url = storage
            .presigned_url("GET", "test.txt", Duration::from_secs(60), now)
            .unwrap();
        let normalized = R2Config {
            endpoint: "https://bucket.example.com".into(),
            ..aws_example_config(R2_REGION)
        };
        assert_eq!(
            url,
            presign(
                &normalized,
                "bucket.example.com",
                "GET",
                "/examplebucket/test.txt",
                60,
                now
            )
        );
    }

    #[test]
    fn configuration_is_checked_and_redacted() {
        let mut config = aws_example_config(R2_REGION);
        config.endpoint = "http://example.com".into();
        assert!(
            R2Storage::new(config).is_err(),
            "plain http is only for local tests"
        );
        let mut config = aws_example_config(R2_REGION);
        config.secret_access_key = " ".into();
        assert!(R2Storage::new(config).is_err());
        let debug = format!("{:?}", aws_example_config(R2_REGION));
        assert!(!debug.contains("wJalrXUtnFEMI") && !debug.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn presigned_urls_are_limited_to_get_and_put_and_a_sane_lifetime() {
        let storage = R2Storage::new(aws_example_config(R2_REGION)).unwrap();
        let now = aws_example_time();
        let key = "tenants/t/attachments/a";
        assert!(
            storage
                .presigned_url("DELETE", key, Duration::from_secs(60), now)
                .is_err()
        );
        assert!(
            storage
                .presigned_url("GET", key, Duration::ZERO, now)
                .is_err()
        );
        assert!(
            storage
                .presigned_url(
                    "GET",
                    key,
                    Duration::from_secs(MAX_PRESIGN_SECONDS + 1),
                    now
                )
                .is_err()
        );
        let url = storage
            .presigned_url("PUT", key, Duration::from_secs(300), now)
            .unwrap();
        assert!(url.starts_with(
            "https://examplebucket.s3.amazonaws.com/examplebucket/tenants/t/attachments/a?"
        ));
        assert!(url.contains("X-Amz-Expires=300"));
    }
}
