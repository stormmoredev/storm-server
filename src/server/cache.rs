use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use fs2::FileExt;
use crate::server::http_stream::HttpStream;
use crate::conf::Conf;

pub struct Cache;

impl Cache {
    pub fn qualifies(path: &str, conf: &Conf) -> bool {
        if !conf.cache_enabled {
            return false;
        }
        conf.cache_patterns.iter().any(|p| path.starts_with(p))
    }

    pub fn key_to_filename(key: &str) -> String {
        key.trim_start_matches('/')
            .replace('/', "_")
            .replace("?", "_")
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
        if !conf.cache_enabled{
            return None;
        }
        let mut cache_request = false;
        let mut delete_prefix: Option<String> = None;

        headers.retain(|(k, v)| {
            if k.eq_ignore_ascii_case("x-cache-path-query") {
                cache_request = true;
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

        if cache_request {
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
        let mut lock = File::create(path)?;
        lock.lock_exclusive()?;
        lock.write_all(buf)?;
        lock.unlock()?;
        Ok(())
    }

    pub async fn send_cached(stream: &mut HttpStream, path: &Path) -> io::Result<()> {
        let mut file = File::open(path)?;
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
                if file_path.is_file() {
                    Cache::send_cached(stream, &file_path).await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
