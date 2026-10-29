// Load file
// Convert to lines
// Convert line to proxy format
// Convert proxy format to desired proxy

use crate::proxies::ProxyError::{FileReadError, ProxyConversionError};
use crate::traits::SplitAtNth;
use itertools::{Either, Itertools};
use std::fmt::{Display, Formatter, Write};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

#[derive(PartialEq, Eq, Debug)]
pub enum ProxyHost {
    HTTP,
    HTTPS,
    SOCKS4,
    SOCKS5,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Proxy {
    host: ProxyHost,
    ip: Arc<str>,
    port: Arc<str>,
    username: Option<Arc<str>>,
    password: Option<Arc<str>>,
}

#[derive(Debug)]
pub enum ProxyError {
    FileReadError(Arc<str>),
    InvalidProxy(Arc<str>),
    ProxyConversionError(Arc<str>),
}

impl Display for Proxy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.username.as_deref(), self.password.as_deref()) {
            (Some(username), Some(password)) => f.write_fmt(format_args!(
                "{}://{username}:{password}@{}:{}",
                self.host, self.ip, self.port
            )),
            _ => f.write_fmt(format_args!("{}://{}:{}", self.host, self.ip, self.port)),
        }
    }
}

impl Display for ProxyHost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyHost::HTTP => f.write_str("http"),
            ProxyHost::HTTPS => f.write_str("https"),
            ProxyHost::SOCKS4 => f.write_str("socks4"),
            ProxyHost::SOCKS5 => f.write_str("socks5")
        }
    }
}

#[cfg(feature = "wreq")]
impl From<Proxy> for wreq::Proxy {
    fn from(value: Proxy) -> Self {
        Self::all(value.to_string()).unwrap()
    }
}

pub fn load_proxies(file_name: &str) -> Result<Vec<Proxy>, Vec<ProxyError>> {
    let file = load_file(file_name).map_err(|e| vec![e])?;
    let lines = split_lines(file).map_err(|e| vec![e])?;
    let (proxies, errors): (Vec<Proxy>, Vec<ProxyError>) =
        lines
            .into_iter()
            .partition_map(|line| match parse_line(&*line) {
                Ok(proxy) => Either::Left(proxy),
                Err(err) => Either::Right(err),
            });

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(proxies)
}

fn load_file(file_name: &str) -> Result<File, ProxyError> {
    File::open(file_name).map_err(|err| {
        let err = err.to_string();
        FileReadError(err.into())
    })
}

fn split_lines(file: File) -> Result<Vec<String>, ProxyError> {
    let buffer = BufReader::new(file);
    Ok(buffer.lines().filter_map(|line| line.ok()).collect())
}

fn parse_line(line: &str) -> Result<Proxy, ProxyError> {
    let host = line.into();

    let removed_host = match line.split_once("://") {
        Some((_, remaining)) => remaining,
        None => line,
    };

    match removed_host.split_once("@") {
        Some((left, right)) => from_split_format(host, left, right),
        None => match removed_host.split_at_nth(':', 2) {
            Some((ip_port, user_pass)) => from_split_format(host, user_pass, ip_port),
            None => from_split_format(host, "", removed_host),
        },
    }
}

fn from_split_format(host: ProxyHost, user_pass: &str, ip_port: &str) -> Result<Proxy, ProxyError> {
    let (user, pass) = user_pass
        .split_once(":")
        .map_or_else(|| (None, None), |(l, r)| (Some(l), Some(r)));
    let (ip, port) = match ip_port.split_once(":") {
        Some((ip, port)) => (ip, port),
        None => {
            let msg = format!("ip:port format invalid, recv: {ip_port}");
            return Err(ProxyConversionError(msg.into()));
        }
    };

    Ok(Proxy {
        host,
        ip: ip.into(),
        port: port.into(),
        username: user.map(|u| u.into()),
        password: pass.map(|p| p.into()),
    })
}

impl From<&str> for ProxyHost {
    fn from(value: &str) -> Self {
        match value.as_bytes() {
            [b'h', b't', b't', b'p', b's', ..] => ProxyHost::HTTPS,
            [b'h', b't', b't', b'p', ..] => ProxyHost::HTTP,
            [b's', b'o', b'c', b'k', b's', b'4', ..] => ProxyHost::SOCKS4,
            [b's', b'o', b'c', b'k', b's', b'5', ..] => ProxyHost::SOCKS5,
            _ => ProxyHost::HTTP,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::proxies::{Proxy, ProxyHost, load_file};

    #[test]
    fn load_proxies() {
        let proxies = crate::proxies::load_proxies("tests/proxies.txt").unwrap();
        assert_eq!(proxies.len(), 5);

        assert_eq!(
            &*proxies,
            &[
                Proxy {
                    host: ProxyHost::HTTP,
                    ip: "1.1.1.1".into(),
                    port: "4444".into(),
                    username: None,
                    password: None,
                },
                Proxy {
                    host: ProxyHost::HTTPS,
                    ip: "1.1.1.1".into(),
                    port: "4444".into(),
                    username: Some("user".into()),
                    password: Some("pass".into()),
                },
                Proxy {
                    host: ProxyHost::HTTP,
                    ip: "1.1.1.1".into(),
                    port: "4444".into(),
                    username: None,
                    password: None,
                },
                Proxy {
                    host: ProxyHost::SOCKS4,
                    ip: "1.1.1.1".into(),
                    port: "4444".into(),
                    username: None,
                    password: None,
                },
                Proxy {
                    host: ProxyHost::SOCKS5,
                    ip: "1.1.1.1".into(),
                    port: "4444".into(),
                    username: Some("user".into()),
                    password: Some("pass".into()),
                }
            ]
        )
    }

    #[test]
    fn load_invalid_file() {
        assert!(load_file("not_real.txt").is_err())
    }

    #[test]
    fn get_http_host_type() {
        let proxy = "http://1.1.1.1:44444";
        let host: ProxyHost = proxy.into();

        assert_eq!(host, ProxyHost::HTTP);
    }

    #[test]
    fn get_https_host_type() {
        let proxy = "https://1.1.1.1:44444";
        let host: ProxyHost = proxy.into();

        assert_eq!(host, ProxyHost::HTTPS);
    }

    #[test]
    fn get_socks4_host_type() {
        let proxy = "socks4://1.1.1.1:44444";
        let host: ProxyHost = proxy.into();

        assert_eq!(host, ProxyHost::SOCKS4);
    }

    #[test]
    fn get_socks5_host_type() {
        let proxy = "socks5://1.1.1.1:44444";
        let host: ProxyHost = proxy.into();

        assert_eq!(host, ProxyHost::SOCKS5);
    }
}
