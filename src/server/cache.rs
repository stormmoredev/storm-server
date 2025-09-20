use std::fs::{self, remove_file, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use fd_lock::RwLock;
use crate::server::http_stream::HttpStream;
use crate::conf::Conf;

pub struct Cache;

impl Cache {

    pub fn process_headers(
        headers: &mut Vec<(String, String)>,
        conf: &Conf,
    ) -> Option<PathBuf> {
        if !conf.cache_enabled{
            return None;
        }
        let mut request_path: Option<String> = None;
        let mut delete_prefix: Option<String> = None;
        let mut delete_path: Option<String> = None;


        headers.retain(|(k, v)| {
            if k.eq_ignore_ascii_case("x-cache-request") {
                request_path = Cache::key_to_filename(v);
                false
            } else if k.eq_ignore_ascii_case("x-cache-delete-like") {
                delete_prefix = Cache::key_to_filename(v);
                false
            }
            else if k.eq_ignore_ascii_case("x-cache-delete") {
                delete_path = Cache::key_to_filename(v);
                false
            }
            else {
                true
            }
        });

        if let Some(prefix) = delete_prefix {
            Cache::delete_like(conf, prefix.as_str());
        }
        if let Some(path) = delete_path {
            Cache::delete(conf, path.as_str());
        }

        if request_path.is_some() {
            if let Some(path) = Cache::file_path(conf, request_path.unwrap().as_ref()) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                return Some(path);
            }
        }
        None
    }

    pub fn qualifies(path: &str, conf: &Conf) -> bool {
        if !conf.cache_enabled {
            return false;
        }
        conf.cache_patterns.iter().any(|p| path.starts_with(p))
    }

    pub fn key_to_filename(key: &str) -> Option<String> {
        let filename = key.trim_start_matches('/')
            .replace('/', "_")
            .replace("?", "_")
            .replace("&", "_")
            .replace("|", "_")
            .replace("<", "_")
            .replace(">", "_")
            .replace("*", "_")
            .replace('"', "_")
            .replace('\\', "_")
            .replace(":", "_");
        Some(filename.to_string())
    }

    pub fn file_path(conf: &Conf, key: &str) -> Option<PathBuf> {
        if let Some(filename) = Cache::key_to_filename(key) {
            return conf.cache_dir.as_ref().map(|dir| dir.join(filename))
        }
        None
    }

    pub fn delete_like(conf: &Conf, like: &str) {
        if !conf.cache_enabled {
            return;
        }
        if let Some(dir) = &conf.cache_dir {
            let prefix = match Cache::key_to_filename(like) {
                Some(prefix) => prefix,
                None => return,
            };
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(&prefix) {
                            let _ = Cache::lock_and_remove(&entry.path());
                        }
                    }
                }
            }
        }
    }

    pub fn delete(conf: &Conf, like: &str) {
        if !conf.cache_enabled {
            return;
        }
        if let Some(dir) = &conf.cache_dir {
            let filename = match Cache::key_to_filename(like) {
                Some(filename) => filename,
                None => return
            };
            let path = dir.join(filename);
            if path.is_file() {
                let _ = Cache::lock_and_remove(&path);
            }
        }
    }

    fn lock_and_remove(path: &Path) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(false)
            .read(true)
            .write(true)
            .open(path)?;

        let mut lock = RwLock::new(file);
        {
            let _ =lock.write()?;
            #[cfg(unix)]
            {
                remove_file(path)?;
            }
        }
        #[cfg(windows)]
        {
            remove_file(path)?;
        }

        Ok(())
    }

    pub fn write(buf: &[u8], path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.is_file() {
            return Ok(());
        }
        let lock_path = path.with_extension(".lock");
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)?;

            let mut lock = RwLock::new(file);
            {
                let mut guard = lock.write()?;
                guard.write_all(buf)?;
            }

            drop(lock);
            if let Err(e) = fs::rename(&lock_path, path) {
                remove_file(&lock_path)?;
                return Err(e);
            }
            Ok(())
        })();

        result
    }

    pub async fn send_cached(stream: &mut HttpStream, path: &Path) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .create_new(false)
            .open(path)?;
        let mut buff = [0; 32 * 1024];
        loop {
            let read = file.read(&mut buff)?;
            if read == 0 { break; }
            stream.write(&buff[..read]).await?;
        }
        Ok(())
    }

    pub async fn try_serve_cached(
        stream: &mut HttpStream,
        path: &str,
        key: &str,
        conf: &Conf,
    ) -> io::Result<bool> {
        if Cache::qualifies(path, conf) {
            if let Some(file_path) = Cache::file_path(conf, key) {
                Cache::send_cached(stream, &file_path).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
