fn quote(s: &str) -> String {
    format!("'{}'", s.replace("'", "'\"'\"'"))
}
