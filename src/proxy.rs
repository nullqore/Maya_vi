use std::sync::mpsc::{self, Receiver};
use std::thread;

pub enum ProxyProgress {
    Advanced(f32),
    Finished,
    Errored(String),
}

pub fn spawn_proxy_thread(
    urls: Vec<String>,
    proxy_address: String,
    threads: u32,
) -> Receiver<ProxyProgress> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let total_urls = urls.len();
        let urls_per_thread = (total_urls as f32 / threads as f32).ceil() as usize;

        let mut thread_handles = Vec::new();

        for chunk in urls.chunks(urls_per_thread) {
            let chunk = chunk.to_vec();
            let proxy_address = proxy_address.clone();
            let sender = sender.clone();

            let handle = thread::spawn(move || {
                for (i, url) in chunk.iter().enumerate() {
                    match send_to_proxy(url, &proxy_address) {
                        Ok(_) => {
                            let progress = (i + 1) as f32 / chunk.len() as f32 * 100.0;
                            let _ = sender.send(ProxyProgress::Advanced(progress));
                        }
                        Err(e) => {
                            let _ = sender.send(ProxyProgress::Errored(e));
                        }
                    }
                }
            });
            thread_handles.push(handle);
        }

        for handle in thread_handles {
            handle.join().unwrap();
        }

        let _ = sender.send(ProxyProgress::Finished);
    });
    receiver
}

pub fn send_to_proxy(url: &str, proxy_address: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .proxy(reqwest::Proxy::all(proxy_address).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;

    client.get(url).send().map_err(|e| e.to_string())?;

    Ok(())
}
