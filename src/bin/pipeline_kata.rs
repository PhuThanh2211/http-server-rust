use std::collections::HashMap;
use std::io;
use std::io::BufRead;

pub struct RouteTable {
    routes: HashMap<String, HashMap<String, String>>, // path -> method -> handler_name
}

impl RouteTable {
    fn new() -> Self {
        RouteTable {
            routes: HashMap::new()
        }
    }

    fn add(&mut self, method: &str, path: &str, handler: &str) {
        self.routes
            .entry(path.to_string())
            .or_insert_with(HashMap::new)
            .insert(method.to_string(), handler.to_string());
    }

    /// Returns (status, Some(handler_name)) on a hit, or (status, None) on 404/405.
    pub fn dispatch(&self, method: &str, path: &str) -> (u16, Option<String>) {
        match self.routes.get(path) {
            None => (404, None),
            Some(methods) => match methods.get(method) {
                Some(handler) => (200, Some(handler.clone())),
                None => (405, None),
            }
        }
    }
}

fn strip_query(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines().map(|l| l.unwrap());
    let mut table = RouteTable::new();

    while let Some(line) = lines.next() {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(4, ' ');
        match parts.next() {
            Some("ROUTE") => {
                let method = parts.next().unwrap_or("");
                let path = parts.next().unwrap_or("");
                let handler = parts.next().unwrap_or("");
                table.add(method, path, handler);
            }
            Some("REQUEST") => {
                let method = parts.next().unwrap_or("");
                let raw_path = parts.next().unwrap_or("");
                let body = parts.next().unwrap_or("");
                process_request(&table, method, raw_path, body);
            }
            _ => {}
        }
    }
}

fn process_request(table: &RouteTable, method: &str, raw_path: &str, body: &str) {
    let path = strip_query(raw_path);
    let body_length = body.len();

    let (status, handler) = table.dispatch(method, path);
    match &handler {
        Some(name) => println!("{} {}", name, body_length),
        None => println!("{}", status),
    }

    println!("LOG {} {} {} {}", method, path, status, body_length);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> RouteTable {
        let mut t = RouteTable::new();
        t.add("GET", "/", "index");
        t.add("POST", "/api/users", "create_user");
        t
    }

    #[test]
    fn exact_hit_returns_200_and_handler() {
        let t = sample_table();
        assert_eq!(t.dispatch("GET", "/"), (200, Some("index".to_string())));
    }

    #[test]
    fn wrong_method_returns_405() {
        let t = sample_table();
        assert_eq!(t.dispatch("GET", "/api/users"), (405, None));
    }

    #[test]
    fn unknown_path_returns_404() {
        let t = sample_table();
        assert_eq!(t.dispatch("GET", "/missing"), (404, None));
    }

    #[test]
    fn strip_query_removes_everything_after_question_mark() {
        assert_eq!(strip_query("/about?ref=footer"), "/about");
        assert_eq!(strip_query("/about"), "/about");
    }
}