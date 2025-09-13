use std::cell::{Ref, RefCell, RefMut};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{PathBuf};
use chrono::Local;
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

pub struct Logger {
    min: usize,
    enabled: bool,
    path: Option<PathBuf>
}

impl Logger {

    pub fn new(path: Option<PathBuf>) -> Logger {
        Logger {
            min: 0,
            enabled: true,
            path
        }
    }
    pub fn clone(&self) -> Logger {
        Logger {
            min: self.min,
            enabled: self.enabled,
            path: self.path.clone()
        }
    }

    pub fn log_d(&self, msg: &str) {
        if self.min == 0{
            self.log( "DEBUG", msg);
        }
    }

    pub fn log_i(&self, msg: &str) {
        if self.min <= 1 {
            self.log("INFO", msg);
        }
    }

    pub fn log_e(&self, msg: &str) {
        if self.min <= 2 {
            self.log( "ERROR", msg);
        }
    }

    fn log(&self, level: &str,  msg: &str) {
        if !self.enabled {
            return;
        }
        let msg = format!("{}| {}| {}",
                          level,
                          msg,
                          Local::now().format("%Y-%m-%d %H:%M:%S"));
        println!("{}", msg);
        if let Some(path) = &self.path {
            let now = Local::now();
            let today = now.format("%Y-%m-%d").to_string();
            let filename = format!("{}.log", today);
            let filepath = path.join(filename);
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(filepath);

            if file.is_ok() {
                let msg = format!("{} \n", msg);
                let mut file = file.unwrap();
                let _ = file.write_all(msg.as_bytes());
            }
        }
    }
}