use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct NodeValue {
    pub is_endpoint: bool,
    pub children: HashSet<String>,
    pub scheme: Option<String>,
}

pub enum Progress {
    Advanced(f32, Option<Duration>, usize),
    Finished(Arc<sled::Db>, usize),
    Errored(String),
}

pub fn get_node_value(db: &sled::Db, key: &str) -> Option<NodeValue> {
    db.get(key)
        .ok()
        .flatten()
        .and_then(|ivec| serde_json::from_slice(&ivec).ok())
}

pub fn get_children(db: &sled::Db, key: &str) -> Vec<String> {
    get_node_value(db, key).map_or(Vec::new(), |v| v.children.into_iter().collect())
}





fn process_url(url: &Url, url_count: &mut usize, cache: &mut HashMap<String, NodeValue>) {
    if let Some(host) = url.host_str() {
        *url_count += 1;

        let root_node = cache.entry("__ROOT__".to_string()).or_default();
        root_node.children.insert(host.to_string());

        let host_node = cache.entry(host.to_string()).or_default();
        if host_node.scheme.as_deref() != Some("https") {
            host_node.scheme = Some(url.scheme().to_string());
        }

        let mut parent_key = host.to_string();
        let path_segments: Vec<String> = url.path_segments().map_or(Vec::new(), |s| s.map(String::from).filter(|s| !s.is_empty()).collect());

        if let Some((last_segment, parent_segments)) = path_segments.split_last() {
            for segment in parent_segments {
                cache.entry(parent_key.clone()).or_default().children.insert(segment.clone());
                parent_key.push('/');
                parent_key.push_str(segment);
            }

            let mut final_leaf_name = last_segment.clone();
            if let Some(query) = url.query() { final_leaf_name.push('?'); final_leaf_name.push_str(query); }
            if let Some(fragment) = url.fragment() { final_leaf_name.push('#'); final_leaf_name.push_str(fragment); }
            cache.entry(parent_key.clone()).or_default().children.insert(final_leaf_name.clone());

            parent_key.push('/');
            parent_key.push_str(&final_leaf_name);
            let endpoint_node = cache.entry(parent_key).or_default();
            endpoint_node.is_endpoint = true;
            endpoint_node.scheme = Some(url.scheme().to_string());

        } else {
            let mut leaf_part = String::new();
            if let Some(query) = url.query() { leaf_part.push('?'); leaf_part.push_str(query); }
            if let Some(fragment) = url.fragment() { leaf_part.push('#'); leaf_part.push_str(fragment); }
            
            let endpoint_key = if !leaf_part.is_empty() {
                cache.entry(parent_key.clone()).or_default().children.insert(leaf_part.clone());
                format!("{}/{}", parent_key, leaf_part)
            } else {
                parent_key
            };
            let endpoint_node = cache.entry(endpoint_key).or_default();
            endpoint_node.is_endpoint = true;
            endpoint_node.scheme = Some(url.scheme().to_string());
        }
    }
}

pub fn spawn_file_processing_thread(path: PathBuf) -> Receiver<Progress> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let db_path = if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.pop();
            exe_path.push("maya.db");
            exe_path
        } else {
            PathBuf::from("maya.db")
        };

        if let Err(e) = std::fs::remove_dir_all(&db_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                sender
                    .send(Progress::Errored(format!("Failed to remove old db: {}", e)))
                    .unwrap();
                return;
            }
        }

        let db = match sled::open(db_path) {
            Ok(db) => db,
            Err(e) => {
                sender
                    .send(Progress::Errored(format!("Failed to open database: {}", e)))
                    .unwrap();
                return;
            }
        };

        let file = match File::open(&path) {
            Ok(file) => file,
            Err(e) => {
                sender
                    .send(Progress::Errored(format!("Failed to open file: {}", e)))
                    .unwrap();
                return;
            }
        };

        let total_size = file.metadata().map(|m| m.len()).unwrap_or(1) as f32;
        let mut reader = io::BufReader::new(file);
        let mut url_count = 0;
        let mut bytes_read = 0.0;
        let start_time = Instant::now();
        let mut last_update = Instant::now();

        let mut buffer = String::new();
        let mut cache: HashMap<String, NodeValue> = HashMap::new();
        const BATCH_SIZE: usize = 10000;

        while let Ok(bytes) = reader.read_line(&mut buffer) {
            if bytes == 0 {
                break;
            }
            bytes_read += bytes as f32;

            let parts = buffer.split(|c| c == '<' || c == '>' || c == '"');
            for part in parts {
                let trimmed_part = part.trim();
                if !trimmed_part.is_empty() {
                    let sanitized_part = trimmed_part.replace(' ', "%20");
                    if let Ok(url) = Url::parse(&sanitized_part) {
                        process_url(&url, &mut url_count, &mut cache);
                    } else if sanitized_part.starts_with("//") {
                        let with_scheme = "https:".to_owned() + &sanitized_part;
                        if let Ok(url) = Url::parse(&with_scheme) {
                            process_url(&url, &mut url_count, &mut cache);
                        }
                    }
                }
            }
            buffer.clear();

            if cache.len() >= BATCH_SIZE {
                if let Err(e) = apply_batch(&db, &cache) {
                    sender
                        .send(Progress::Errored(format!("DB batch apply error: {}", e)))
                        .unwrap();
                    return;
                }
                cache.clear();

                if last_update.elapsed() > Duration::from_millis(100) {
                    let elapsed_secs = start_time.elapsed().as_secs_f32();
                    let speed = if elapsed_secs > 0.0 { bytes_read / elapsed_secs } else { 0.0 };
                    let remaining_bytes = total_size - bytes_read;
                    
                    let remaining_time = if speed > 0.0 && remaining_bytes > 0.0 {
                        Duration::from_secs_f32(remaining_bytes / speed)
                    } else {
                        Duration::from_secs(0)
                    };

                    sender
                        .send(Progress::Advanced(
                            (bytes_read / total_size) * 100.0,
                            Some(remaining_time),
                            url_count,
                        ))
                        .unwrap();
                    last_update = Instant::now();
                }
            }
        }

        if !cache.is_empty() {
            if let Err(e) = apply_batch(&db, &cache) {
                sender
                    .send(Progress::Errored(format!(
                        "DB final batch apply error: {}",
                        e
                    )))
                    .unwrap();
                return;
            }
        }
        
        db.flush().unwrap();

        sender
            .send(Progress::Finished(Arc::new(db), url_count))
            .unwrap();
    });

    receiver
}

fn apply_batch(
    db: &sled::Db,
    cache: &HashMap<String, NodeValue>,
) -> Result<(), sled::Error> {
    let mut batch = sled::Batch::default();
    for (key, value) in cache.iter() {
        let existing_value: NodeValue = db
            .get(key)
            .unwrap_or(None)
            .and_then(|ivec| serde_json::from_slice(&ivec).ok())
            .unwrap_or_default();

        let mut new_value = value.clone();
        new_value.is_endpoint = new_value.is_endpoint || existing_value.is_endpoint;
        new_value.children.extend(existing_value.children);

        let encoded = serde_json::to_vec(&new_value).unwrap();
        batch.insert(key.as_bytes(), encoded);
    }
    db.apply_batch(batch)
}