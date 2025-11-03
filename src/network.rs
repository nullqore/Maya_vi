use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

pub type NetworkResult = (String, String, String, String, String, u64, u128);

pub fn spawn_request_thread(url: String) -> Receiver<NetworkResult> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let start_time = Instant::now();
        let result = match reqwest::blocking::get(&url) {
            Ok(response) => {
                let request_str = format!("GET {} HTTP/1.1\nHost: {}\nUser-Agent: Sitemapper/1.0\nAccept: */*\n",
                    url.splitn(4, '/').nth(3).unwrap_or(""),
                    url.splitn(4, '/').nth(2).unwrap_or("-"));

                let status = response.status();
                let headers = response.headers().clone();
                let content_length = response.content_length().unwrap_or(0);
                let content_type = headers
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|val| val.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();

                let body = response.text().unwrap_or_else(|e| format!("Failed to read response body: {}", e));
                let elapsed = start_time.elapsed().as_millis();

                let (pretty_body, language) = if content_type.contains("application/json") {
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(json_value) => {
                            let pretty_json = serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| body.clone());
                            (pretty_json, "json".to_string())
                        }
                        Err(_) => (body.clone(), "json".to_string()),
                    }
                } else if content_type.contains("text/html") {
                    (body.clone(), "html".to_string())
                } else if content_type.contains("text/xml") || content_type.contains("application/xml") {
                    (body.clone(), "xml".to_string())
                } else if content_type.contains("javascript") {
                    (body.clone(), "javascript".to_string())
                } else {
                    (body.clone(), "text".to_string())
                };

                let response_headers_str = format!("HTTP/1.1 {}\n{:#?}", status, headers);
                (request_str, response_headers_str, body, pretty_body, language, content_length, elapsed)
            }
            Err(e) => (
                format!("Failed to make request to: {}", url),
                String::new(),
                format!("Error: {:#?}", e),
                String::new(),
                "text".to_string(),
                0,
                start_time.elapsed().as_millis(),
            ),
        };
        let _ = sender.send(result);
    });

    receiver
}
