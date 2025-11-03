use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

pub enum SaveProgress {
    Finished,
    Errored(String),
}

pub fn spawn_file_saving_thread(db: Arc<sled::Db>, path: PathBuf) -> Receiver<SaveProgress> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut endpoints = Vec::new();
        for item in db.iter() {
            if let Ok((key, _value)) = item {
                if let Ok(key_str) = std::str::from_utf8(&key) {
                    endpoints.push(key_str.to_string());
                }
            }
        }

        match File::create(&path) {
            Ok(mut file) => {
                endpoints.sort();
                for url in endpoints {
                    if url != "__ROOT__" {
                        if let Err(e) = writeln!(file, "{}", url) {
                            let _ = sender.send(SaveProgress::Errored(format!(
                                "Failed to write to file: {}",
                                e
                            )));
                            return;
                        }
                    }
                }
                let _ = sender.send(SaveProgress::Finished);
            }
            Err(e) => {
                let _ =
                    sender.send(SaveProgress::Errored(format!("Failed to create file: {}", e)));
            }
        }
    });
    receiver
}
