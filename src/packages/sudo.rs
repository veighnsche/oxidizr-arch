pub fn applets() -> Vec<String> {
    [
        "sudo",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
