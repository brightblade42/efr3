pub(crate) fn normalize_endpoint(endpoint: String) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    if endpoint.contains("://") {
        endpoint
    } else {
        format!("http://{}", endpoint)
    }
}
