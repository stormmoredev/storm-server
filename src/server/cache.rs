use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::server::http_stream::HttpStream;

use crate::conf::Conf;
use std::thread::sleep;
use std::time::Duration;

pub struct Cache;

pub struct CacheStatistic {
    pub size: i32,
}

impl CacheStatistic {
    pub fn new() -> CacheStatistic {
        CacheStatistic { size: 0 }
    }

    pub fn recalculate_files() -> Vec<String> {
        vec![]
    }
}

impl Cache {
    pub fn qualifies(path: &str, conf: &Conf) -> bool {
        if !conf.cache_enabled {
            return false;
        }
        conf.cache_patterns.iter().any(|p| path.starts_with(p))
    }

    pub fn key_to_filename(key: &str) -> String {
        key.trim_start_matches('/')
            .replace('/', "-")
    }

    pub fn file_path(conf: &Conf, key: &str) -> Option<PathBuf> {
        conf.cache_dir.as_ref().map(|dir| dir.join(Self::key_to_filename(key)))
    }

    pub fn delete_like(conf: &Conf, like: &str) {
        if !conf.cache_enabled {
            return;
        }
        if let Some(dir) = &conf.cache_dir {
            let prefix = Self::key_to_filename(like);
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(&prefix) {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    pub fn process_headers(
        headers: &mut Vec<(String, String)>,
        query_path: &str,
        conf: &Conf,
    ) -> Option<PathBuf> {
        let mut cache_enabled = false;
        let mut delete_prefix: Option<String> = None;

        headers.retain(|(k, v)| {
            if k.eq_ignore_ascii_case("x-cache-path-query") {
                cache_enabled = true;
                false
            } else if k.eq_ignore_ascii_case("x-cache-delete-like") {
                delete_prefix = Some(v.clone());
                false
            } else {
                true
            }
        });

        if let Some(prefix) = delete_prefix {
            Cache::delete_like(conf, prefix.as_str());
        }

        if cache_enabled {
            if let Some(path) = Cache::file_path(conf, query_path) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                return Some(path);
            }
        }
        None
    }

    pub fn write(buf: &[u8], path: &Path) -> io::Result<()> {
        let lock_path = path.with_extension("lock");
        // naive spin-lock using lock file creation
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&lock_path) {
                Ok(lock_file) => {
                    // once lock acquired, write data and release
                    let res = (|| {
                        let mut file = File::create(path)?;
                        file.write_all(buf)
                    })();
                    let _ = fs::remove_file(&lock_path);
                    drop(lock_file);
                    return res;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn send_cached(stream: &mut HttpStream, path: &Path) -> io::Result<()> {
        let mut file = File::open(path)?;
        let mut buff = [0; 256 * 1024];
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
                if file_path.is_file() {
                    Cache::send_cached(stream, &file_path).await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
