use super::BrandError;
use std::{
    io::{self, Read},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    sync::mpsc,
    time::{Duration, Instant},
};
use url::{Host, Url};

pub(crate) const MAX_HTML_BYTES: usize = 256 * 1024;
pub(crate) const MAX_IMAGE_BYTES: usize = 1024 * 1024;

fn blocked() -> BrandError {
    BrandError::new(
        "brand/blocked",
        "Brand links must use a public HTTP(S) address and standard web port",
    )
}
fn unavailable() -> BrandError {
    BrandError::new(
        "brand/capture-unavailable",
        "The brand resource could not be fetched before the capture deadline",
    )
}

pub(crate) fn normalize_url(value: &str) -> Result<Url, BrandError> {
    let url = Url::parse(value).map_err(|_| blocked())?;
    let expected = match url.scheme() {
        "https" => 443,
        "http" => 80,
        _ => return Err(blocked()),
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(expected)
    {
        return Err(blocked());
    }
    match url.host().ok_or_else(blocked)? {
        Host::Domain(host) => {
            let host = host.trim_end_matches('.').to_ascii_lowercase();
            if host == "localhost"
                || host.ends_with(".localhost")
                || host == "local"
                || host.ends_with(".local")
            {
                return Err(blocked());
            }
        }
        Host::Ipv4(ip) if !public_address(ip.into()) => return Err(blocked()),
        Host::Ipv6(ip) if !public_address(ip.into()) => return Err(blocked()),
        _ => {}
    }
    Ok(url)
}

pub(crate) fn public_url(url: &Url) -> String {
    let mut url = url.clone();
    url.set_query(None);
    url.set_fragment(None);
    url.into()
}

fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 0 && matches!(c, 0 | 2))
                || (a == 192 && b == 88 && c == 99)
                || (a == 192 && b == 168)
                || (a == 198 && matches!(b, 18 | 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let [a, b, ..] = ip.segments();
            // Only global unicast, excluding transition mechanisms, protocol
            // assignments and documentation. IPv4-compatible/mapped forms are
            // excluded by the unicast prefix, including hex encodings.
            (a & 0xe000) == 0x2000
                && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
                && a != 0x2002
                && !(a == 0x3fff && (b & 0xf000) == 0)
        }
    }
}

fn remaining(deadline: Instant) -> Result<Duration, BrandError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(unavailable)
}

fn resolve_deadline(
    lookup: impl FnOnce() -> io::Result<Vec<SocketAddr>> + Send + 'static,
    deadline: Instant,
) -> Result<Vec<SocketAddr>, BrandError> {
    let duration = remaining(deadline)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    // A timed-out platform resolver cannot be cancelled. This detached task
    // owns no client/cache state and does not keep the capture process alive.
    std::thread::Builder::new()
        .name("brand-dns".into())
        .spawn(move || {
            let _ = sender.send(lookup());
        })
        .map_err(|_| unavailable())?;
    receiver
        .recv_timeout(duration)
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())
}

fn validate_addresses(addresses: &[SocketAddr]) -> Result<(), BrandError> {
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !public_address(address.ip()))
    {
        return Err(blocked());
    }
    Ok(())
}

fn resolve(url: &Url, deadline: Instant) -> Result<Vec<SocketAddr>, BrandError> {
    let port = url.port_or_known_default().ok_or_else(blocked)?;
    let addresses = match url.host().ok_or_else(blocked)? {
        Host::Ipv4(ip) => vec![SocketAddr::new(ip.into(), port)],
        Host::Ipv6(ip) => vec![SocketAddr::new(ip.into(), port)],
        Host::Domain(host) => {
            let host = host.to_string();
            resolve_deadline(
                move || {
                    (host.as_str(), port)
                        .to_socket_addrs()
                        .map(Iterator::collect)
                },
                deadline,
            )?
        }
    };
    validate_addresses(&addresses)?;
    Ok(addresses)
}

pub(crate) struct Response {
    pub url: Url,
    pub media_type: String,
    response: ureq::Response,
}

fn agent(addresses: Vec<SocketAddr>, deadline: Instant) -> Result<ureq::AgentBuilder, BrandError> {
    let remaining = remaining(deadline)?;
    Ok(ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .redirects(0)
        .timeout(remaining)
        .timeout_connect(remaining)
        // ureq receives exactly the addresses just validated; it never looks
        // up the host again. The URL retains the hostname for Host and SNI.
        .resolver(move |_: &str| Ok(addresses.clone())))
}

fn send(
    url: &Url,
    addresses: Vec<SocketAddr>,
    deadline: Instant,
) -> Result<ureq::Response, BrandError> {
    request(agent(addresses, deadline)?.build(), url)
}
fn request(agent: ureq::Agent, url: &Url) -> Result<ureq::Response, BrandError> {
    match agent
        .get(url.as_str())
        .set(
            "Accept",
            "text/html,application/xhtml+xml,image/png,image/jpeg,image/webp,image/x-icon",
        )
        .set("Accept-Encoding", "identity")
        .set("User-Agent", "Phoenix-Diagram/0.2 brand-capture")
        .call()
    {
        Ok(response) | Err(ureq::Error::Status(_, response)) => Ok(response),
        Err(_) => Err(unavailable()),
    }
}

pub(crate) fn get(url: Url, deadline: Instant) -> Result<Response, BrandError> {
    get_with(url, deadline, &resolve, &send)
}

fn get_with(
    mut url: Url,
    deadline: Instant,
    resolver: &impl Fn(&Url, Instant) -> Result<Vec<SocketAddr>, BrandError>,
    transport: &impl Fn(&Url, Vec<SocketAddr>, Instant) -> Result<ureq::Response, BrandError>,
) -> Result<Response, BrandError> {
    for redirects in 0..=3 {
        remaining(deadline)?;
        url = normalize_url(url.as_str())?;
        let addresses = resolver(&url, deadline)?;
        let response = transport(&url, addresses, deadline)?;
        if matches!(response.status(), 301 | 302 | 303 | 307 | 308) {
            if redirects == 3 {
                return Err(unavailable());
            }
            let location = response.header("Location").ok_or_else(unavailable)?;
            url = url.join(location).map_err(|_| blocked())?;
            continue;
        }
        if !(200..300).contains(&response.status()) {
            return Err(unavailable());
        }
        if response.header("Content-Encoding").is_some_and(|value| {
            !value.trim().is_empty() && !value.trim().eq_ignore_ascii_case("identity")
        }) {
            return Err(BrandError::new(
                "brand/invalid-image",
                "Encoded brand responses are not supported",
            ));
        }
        let media_type = response
            .header("Content-Type")
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        return Ok(Response {
            url,
            media_type,
            response,
        });
    }
    Err(unavailable())
}

impl Response {
    pub fn read_head(self, mut ended: impl FnMut(u8) -> bool) -> Result<(), BrandError> {
        let mut reader = self.response.into_reader();
        let mut chunk = [0; 4096];
        let mut total = 0;
        loop {
            let count = reader.read(&mut chunk).map_err(|_| unavailable())?;
            if count == 0 {
                return Ok(());
            }
            for &byte in &chunk[..count] {
                total += 1;
                if total > MAX_HTML_BYTES {
                    return Err(BrandError::new(
                        "brand/invalid-image",
                        "The brand page head exceeds its byte limit",
                    ));
                }
                if ended(byte) {
                    return Ok(());
                }
            }
        }
    }

    pub fn read(self, maximum: usize) -> Result<Vec<u8>, BrandError> {
        if self
            .response
            .header("Content-Length")
            .and_then(|length| length.parse::<usize>().ok())
            .is_some_and(|length| length > maximum)
        {
            return Err(BrandError::new(
                "brand/invalid-image",
                "The brand resource exceeds its byte limit",
            ));
        }
        let mut bytes = Vec::new();
        self.response
            .into_reader()
            .take((maximum + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable())?;
        if bytes.len() > maximum {
            return Err(BrandError::new(
                "brand/invalid-image",
                "The brand resource exceeds its byte limit",
            ));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reserved_and_mapped_addresses() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.0.1",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "2002::1",
            "64:ff9b::a00:1",
            "::ffff:127.0.0.1",
            "::ffff:7f00:1",
            "::a00:1",
        ] {
            assert!(
                !public_address(address.parse().unwrap()),
                "accepted {address}"
            );
        }
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(
                public_address(address.parse().unwrap()),
                "rejected {address}"
            );
        }
    }
    #[test]
    fn rejects_unsafe_urls_without_leaking_secrets() {
        for value in [
            "file:///tmp/icon.png",
            "data:image/png,secret",
            "https://user:secret@public.example/icon?token=secret",
            "https://public.example:8443/icon",
            "http://localhost/icon",
            "http://name.local/icon",
            "http://2130706433/icon",
            "http://0x7f000001/icon",
            "http://[::ffff:7f00:1]/icon",
        ] {
            let error = normalize_url(value).unwrap_err();
            assert_eq!(error.code, "brand/blocked");
            assert!(!error.message.contains("secret"));
        }
        let url = normalize_url("https://public.example/icon?token=secret#private").unwrap();
        assert_eq!(public_url(&url), "https://public.example/icon");
        assert!(normalize_url("https://public.example:443/icon").is_ok());
    }

    #[test]
    fn all_dns_answers_must_be_public_and_dns_has_its_own_deadline() {
        let addresses = [
            "8.8.8.8:443".parse().unwrap(),
            "127.0.0.1:443".parse().unwrap(),
        ];
        assert_eq!(
            validate_addresses(&addresses).unwrap_err().code,
            "brand/blocked"
        );
        assert!(validate_addresses(&[]).is_err());
        let (release, lookup_gate) = mpsc::channel::<()>();
        let (result_tx, result_rx) = mpsc::channel();
        let caller = std::thread::spawn(move || {
            let result = resolve_deadline(
                move || {
                    let _ = lookup_gate.recv();
                    Ok(vec!["8.8.8.8:443".parse().unwrap()])
                },
                Instant::now() + Duration::from_millis(20),
            );
            let _ = result_tx.send(result);
        });
        // The lookup stays blocked until after the caller returns. The outer
        // timeout only prevents a broken deadline from hanging this test.
        let result = result_rx.recv_timeout(Duration::from_secs(5));
        drop(release);
        caller.join().unwrap();
        let error = result
            .expect("DNS deadline waited for lookup completion")
            .unwrap_err();
        assert_eq!(error.code, "brand/capture-unavailable");

        let (started_tx, started_rx) = mpsc::channel();
        let expired = Instant::now();
        let error = resolve_deadline(
            move || {
                let _ = started_tx.send(());
                Ok(vec!["8.8.8.8:443".parse().unwrap()])
            },
            expired,
        )
        .unwrap_err();
        assert_eq!(error.code, "brand/capture-unavailable");
        assert_eq!(started_rx.try_recv(), Err(mpsc::TryRecvError::Disconnected));
    }

    #[test]
    fn redirects_revalidate_before_dial_and_are_bounded() {
        use std::cell::Cell;
        let resolutions = Cell::new(0);
        let connections = Cell::new(0);
        let resolver = |_: &Url, _: Instant| {
            resolutions.set(resolutions.get() + 1);
            Ok(vec!["8.8.8.8:443".parse().unwrap()])
        };
        let transport = |_: &Url, addresses: Vec<SocketAddr>, _: Instant| {
            assert_eq!(addresses, vec!["8.8.8.8:443".parse().unwrap()]);
            connections.set(connections.get() + 1);
            Ok(ureq::Response::new(302, "Found", "").unwrap())
        };
        let error = get_with(normalize_url("https://public.example/icon").unwrap(), Instant::now() + Duration::from_secs(1), &resolver, &|url, addresses, deadline| {
            let _ = transport(url, addresses, deadline)?;
            let response = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/private\r\nContent-Length: 0\r\n\r\n".parse().unwrap();
            Ok(response)
        }).err().unwrap();
        assert_eq!(error.code, "brand/blocked");
        assert_eq!(resolutions.get(), 1);
        assert_eq!(connections.get(), 1);
        resolutions.set(0);
        connections.set(0);
        let error = get_with(
            normalize_url("https://public.example/icon").unwrap(),
            Instant::now() + Duration::from_secs(1),
            &resolver,
            &|_, _, _| {
                connections.set(connections.get() + 1);
                Ok(
                    "HTTP/1.1 302 Found\r\nLocation: /again\r\nContent-Length: 0\r\n\r\n"
                        .parse()
                        .unwrap(),
                )
            },
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "brand/capture-unavailable");
        assert_eq!(resolutions.get(), 4);
        assert_eq!(connections.get(), 4);
    }

    #[test]
    fn real_http_uses_pinned_socket_and_original_host() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let count = socket.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]);
            assert!(request.contains("Host: pinned-brand.invalid"));
            assert!(request.contains("Accept-Encoding: identity"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 3\r\nConnection: close\r\n\r\npng").unwrap();
        });
        let response = get_with(
            normalize_url("http://pinned-brand.invalid/icon").unwrap(),
            Instant::now() + Duration::from_secs(2),
            &|_, _| Ok(vec![address]),
            &send,
        )
        .unwrap();
        assert_eq!(response.media_type, "image/png");
        assert_eq!(response.read(3).unwrap(), b"png");
        worker.join().unwrap();
    }

    #[test]
    fn rejects_compression_and_declared_or_streamed_oversize_bodies() {
        let deadline = Instant::now() + Duration::from_secs(1);
        let resolver = |_: &Url, _: Instant| Ok(vec!["8.8.8.8:443".parse().unwrap()]);
        let error = get_with(
            normalize_url("https://public.example/icon").unwrap(),
            deadline,
            &resolver,
            &|_, _, _| {
                Ok(
                    "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 0\r\n\r\n"
                        .parse()
                        .unwrap(),
                )
            },
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "brand/invalid-image");
        for raw in [
            "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n",
            "HTTP/1.1 200 OK\r\n\r\n123456789",
        ] {
            let response = get_with(
                normalize_url("https://public.example/icon").unwrap(),
                deadline,
                &resolver,
                &|_, _, _| Ok(raw.parse().unwrap()),
            )
            .unwrap();
            assert_eq!(response.read(8).unwrap_err().code, "brand/invalid-image");
        }
    }

    #[test]
    fn real_tls_pins_socket_preserves_sni_and_rejects_untrusted_or_wrong_hostname() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use std::{io::Write, net::TcpListener, sync::Arc};
        use ureq::rustls::{
            self,
            pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
        };
        // Synthetic self-signed test certificate and key, never production trust.
        let cert = CertificateDer::from(STANDARD.decode("MIIDZjCCAk6gAwIBAgIUCFAlk4FQh6unTCI3qxgK+y3jO0gwDQYJKoZIhvcNAQELBQAwHzEdMBsGA1UEAwwUcGlubmVkLWJyYW5kLmludmFsaWQwIBcNMjYxMDI4MTUzOTAzWhgPMjEyNjEwMDQxNTM5MDNaMB8xHTAbBgNVBAMMFHBpbm5lZC1icmFuZC5pbnZhbGlkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu9bi7m6rr8eJ8DPiSy/v4Jh3fiRQsD9s9/YVJXEjYSDMesksVOMfZKsaoGstYPD0nQVm/RBhEvbzyTEkRYqwc2cvml+rStDsMvV/OB42GVq/8rSHSui12CaT+1/ptUjkO/xhSWDOR4Bjn2P0hlKVttKxL49Fx5R+dglnpbFR9rZkv8YZnDpXCR8nKhJUsIv/zvB4lQKDKhejXlJ+JQmC66WbiyuBuG9cc+QElVCX37vkTtpuf3UEwBwcM+9UFjFUT7wK+4TLwgim/ylRCl68/HWGiaYguy70WXp0INi1RmXm84aYxjGMlcNv0VpHHS+5vAlgAYaFFWe5Y3E1LlXJQQIDAQABo4GXMIGUMB0GA1UdDgQWBBRcb8BzSGX7I0M9Dj0xITGYM43OszAfBgNVHSMEGDAWgBRcb8BzSGX7I0M9Dj0xITGYM43OszAfBgNVHREEGDAWghRwaW5uZWQtYnJhbmQuaW52YWxpZDAMBgNVHRMBAf8EAjAAMA4GA1UdDwEB/wQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDATANBgkqhkiG9w0BAQsFAAOCAQEAf7rIAt35nrslflmUHfPMQPcdSNX9/jLBeBqXOz+8GRUNvVzidnxkplotqd0ad477gaAcUY9owT01JgmfaK3ZXtjv4P95PNTT47R4vSlpzL4tFcEHdl9fc1tq9bAshLohJGSo0BpYoRZbofu9lbfQFUwGj8E12ulxaOjjVke6uLeFA0zGrtgaIK//+zqh9x36NseTNsj5jp4iKvRXrDcrEtZqmjD6vYgq0a8+NG92jSqd32gH5fcfGRwq3AXNdFhDBIkwcwpuSalPWx9wnCK2EfQImVi29a8Z1MA4XlvZLMCe5K3ofyOT1Ooad/XYVNNalkl6AkgWWpa/vYv7HNR5oA==").unwrap());
        let key = STANDARD.decode("MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC71uLubquvx4nwM+JLL+/gmHd+JFCwP2z39hUlcSNhIMx6ySxU4x9kqxqgay1g8PSdBWb9EGES9vPJMSRFirBzZy+aX6tK0Owy9X84HjYZWr/ytIdK6LXYJpP7X+m1SOQ7/GFJYM5HgGOfY/SGUpW20rEvj0XHlH52CWelsVH2tmS/xhmcOlcJHycqElSwi//O8HiVAoMqF6NeUn4lCYLrpZuLK4G4b1xz5ASVUJffu+RO2m5/dQTAHBwz71QWMVRPvAr7hMvCCKb/KVEKXrz8dYaJpiC7LvRZenQg2LVGZebzhpjGMYyVw2/RWkcdL7m8CWABhoUVZ7ljcTUuVclBAgMBAAECggEAWXVh5oQ2qCS0AP30zDIsYB686gqJkybh0sYUFBzp6JwiRmgBAPyaOutyZuwkVILtPMsR3Xwrf0i1miAZUjVXC98cwLzHa35Mv4hD6ooY4cJioEhyFq71y9ezqohnWbs5V4g/HR9Jnqv6xjJohoBWZRe64xes9/w0riqUfkkQ7BnoyJ/V0UrYRPH+KKCl6OzbdsxFFQhRZcs1FnXwElM/1gWg2LkotSWns1WvfKt3ts4krQpefnGSs48cvznnnfrheA1DqDLOaKLaF9SktvVC8wP82t09+IzHOLLQsu4swWeI4jcv8WrY+ceO7fR+VRQCCC0y0EOF7cGR/s+7hlNtcQKBgQDcveV6EiwYk1WL+xgZOAYxSBBVObXW1GqE1XWes4SAa9bnxL4Rq7d3qDRxKmo61cOpQVBoIZPC7Hi4+6HtsNfJD9WRqwXwmzBdLXVfWMyqMKvSsPr3LcKoaQMlHq5SeRQzXh9qiq3WiZsYVXD0mRkkuB+fCvWCyCkSsDgDhjLukwKBgQDZ152kaxzD29WWonAwTZ680Dx14/+ezO2QKuBMAKp90E5VkHmQm+fJS2VGj+ptr8xe6yoQyv+YVPO7Sjn8uP1K7A2/UIIVKSGPDfs7L8QwMIy2tcbQAUco9DjlhEp5gi/KzBfdv+/kVZPIIkj0ecRwPHKJxK3lDzWIWYMwfdr5WwKBgQCUDlZ9cvTaou+tDQST3azhnxk9YJDjf+PWJqpWgMftrucc2yw1A/zxqANMqAtKXRh4TNOWlHbcNemfAUASAhwpwpIXuyMnx58/z/66rf/QYwbMMWKTKcE3Xf/7BMIdlRKEhawCtPN2QyOgxaYWmR1xMIIdFaeoSYKrA0D+tCMzOwKBgQCdUdJ96ADpDTytwXel2ph1oSjYtqRnKXWy9NPw71Yw8BtyPZkdLwpwW6uzDsHtHtVIAyro38FAYLF4I33HHk2GKv0umdSntYGPzzgNIUzcxGMKnDINC685S04/qo8iOzuKiGP/99omGwtTsDbhn4KWz4CkL86pqpctdf+BKJLYGwKBgDnulhyH3ou/fRFj07f2iFzQ59oARpwmCAh6Z+RKGkWsCiPXhkcAexn+ezEjthgNBq5EZMENXXMk3evBK/1/1ijcO3A4v375BXlhlzxm9t9pMkT+LxKqhK8/aduZ+njLHwFyub/dzEMvdIuqZVSGRh2TOVEi329as+ZETzN5UAtF").unwrap();
        for (hostname, trusted, expected) in [
            ("pinned-brand.invalid", true, true),
            ("wrong-brand.invalid", true, false),
            ("pinned-brand.invalid", false, false),
        ] {
            let server_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert.clone()],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.clone())),
                )
                .unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = std::thread::spawn(move || {
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let connection = rustls::ServerConnection::new(Arc::new(server_config)).unwrap();
                let mut stream = rustls::StreamOwned::new(connection, socket);
                let mut buffer = [0; 4096];
                let result = stream.read(&mut buffer);
                if let Ok(count) = result {
                    if count > 0 {
                        let request = String::from_utf8_lossy(&buffer[..count]);
                        assert!(request.contains("Host: pinned-brand.invalid"));
                        assert_eq!(stream.conn.server_name(), Some("pinned-brand.invalid"));
                        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 3\r\nConnection: close\r\n\r\npng").unwrap();
                        return true;
                    }
                }
                false
            });
            let deadline = Instant::now() + Duration::from_secs(2);
            let url = normalize_url(&format!("https://{hostname}/icon")).unwrap();
            let result = get_with(
                url,
                deadline,
                &|_, _| Ok(vec![address]),
                &|url, addresses, deadline| {
                    let builder = agent(addresses, deadline)?;
                    if trusted {
                        let mut roots = rustls::RootCertStore::empty();
                        roots.add(cert.clone()).unwrap();
                        let config = rustls::ClientConfig::builder()
                            .with_root_certificates(roots)
                            .with_no_client_auth();
                        request(builder.tls_config(Arc::new(config)).build(), url)
                    } else {
                        request(builder.build(), url)
                    }
                },
            );
            assert_eq!(
                result.is_ok(),
                expected,
                "TLS acceptance for {hostname}, trusted={trusted}"
            );
            if let Ok(response) = result {
                assert_eq!(response.read(3).unwrap(), b"png");
            }
            assert_eq!(worker.join().unwrap(), expected);
        }
    }
}
