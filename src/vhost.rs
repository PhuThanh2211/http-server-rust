use std::collections::HashMap;

pub struct VHosts {
    exact: HashMap<String, String>,
    wildcards: Vec<(String, String)>, // (".example.com", site_id)
}

pub enum Resolved<'a> {
    Site(&'a str),
    BadRequest, // empty or missing host
    NotFound,
}

pub fn normalize_host(raw: &str) -> String {
    let h = raw.trim().to_ascii_lowercase();
    if h.starts_with('[') {
        return match h.find(']') {
            Some(end) => h[..end].to_string(),
            None => h,
        };
    }
    match h.split_once(':') {
        Some((name, port)) => name.to_string(),
        None => h,
    }
}

impl VHosts {
    pub fn new() -> Self {
        Self {
            exact: HashMap::new(),
            wildcards: Vec::new(),
        }
    }

    pub fn add(&mut self, pattern: &str, site: &str) {
        let p = pattern.trim().to_ascii_lowercase();
        if let Some(suffix) = p.strip_prefix('*') {
            // keep the leading dot so "*.example.com" can't match "example.com"
            self.wildcards.push((suffix.to_string(), site.to_string()))
        } else {
            self.exact.insert(p, site.to_string());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcards.is_empty()
    }

    pub fn resolve(&self, host_header: &str) -> Resolved<'_> {
        let host = normalize_host(host_header);
        if host.is_empty() {
            return Resolved::BadRequest;
        }
        if let Some(site) = self.exact.get(&host) {
            return Resolved::Site(site);
        }

        self.wildcards.iter()
            .filter(|(suffix, _)| host.len() > suffix.len() && host.ends_with(suffix.as_str()))
            .max_by_key(|(suffix, _)| suffix.len()) // longest literal suffix wins
            .map(|(_, site)| Resolved::Site(site.as_str()))
            .unwrap_or(Resolved::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> VHosts {
        let mut v = VHosts::new();
        v.add("blog.example.com", "blog");
        v.add("*.example.com", "wildcard");
        v.add("*.api.example.com", "api");
        v
    }

    fn site(v: &VHosts, h: &str) -> String {
        match v.resolve(h) {
            Resolved::Site(s) => s.to_string(),
            Resolved::BadRequest => "400".into(),
            Resolved::NotFound => "404".into(),
        }
    }

    #[test] fn exact_beats_wildcard() { assert_eq!(site(&table(), "blog.example.com"), "blog"); }
    #[test] fn case_insensitive() { assert_eq!(site(&table(), "BLOG.EXAMPLE.COM"), "blog"); }
    #[test] fn strips_port() { assert_eq!(site(&table(), "shop.example.com:8080"), "wildcard"); }
    #[test] fn wildcard_not_apex() { assert_eq!(site(&table(), "example.com"), "404"); }
    #[test] fn longest_suffix_wins() { assert_eq!(site(&table(), "v1.api.example.com"), "api"); }
    #[test] fn empty_is_400() { assert_eq!(site(&table(), ""), "400"); }
    #[test] fn ipv6_brackets() { assert_eq!(normalize_host("[::1]:8080"), "[::1]"); }
}