use crate::conf::Conf;
use crate::server::http_server::response::string_reader::StringReader;
use crate::server::http_server::response::Response;
use std::collections::HashMap;
use std::path::PathBuf;
use urlencoding::encode;

struct DirItem {
    pub name: String,
    pub kind: u8,
    pub size: u64,
}

impl Response {
    pub fn dir(path: &PathBuf, query_path: &str, conf: &Conf) -> Response {
        let mut list = String::new();

        let mut directory = String::new();
        directory.push_str(query_path);

        let mut parent_directory = String::new();
        if !query_path.eq("/") {
            let last = query_path.rfind("/").unwrap();
            let mut href = &query_path[..last];
            if href.is_empty() {
                href = "/";
            }

            parent_directory.push_str(format!("<a class=\"up\" href=\"{}\">Back</a>", href).as_str());
        }

        let entries = Self::get_dir_items(path);

        for item in entries {
            let name = item.name;
            let encoded_name = encode(name.as_str());
            let encoded_name = encoded_name.as_ref();

            let mut href = String::new();
            href.push_str(query_path);
            if !query_path.ends_with("/") {
                href.push_str("/");
            }
            href.push_str(encoded_name);

            let mut html = String::new();
            if item.kind == 0 {
                let row = format!("<tr><td><a href=\"{}\">{}...</a></td><td>DIR</td><td></td></tr>",
                                  href, name);
                html.push_str(row.as_str());
            }
            if item.kind == 1 {
                let mut size = item.size / 1024;
                let mut unit = "KB";
                if size > 1024 {
                    size = size / 1024;
                    unit = "MB";
                }
                let row = format!("<tr><td><a href=\"{}\">{}</a></td><td></td><td>{} {}</td></tr>",
                                  href, name, size, unit);
                html.push_str(row.as_str());
            }
            list.push_str(&html);
        }

        let version = env!("CARGO_PKG_VERSION");
        let name = conf.domain.clone();
        
        let mut body = String::new();
        body.push_str(include_str!("../request_handler/templates/directory.html"));
        body = body.replace("%list%", &list);
        body = body.replace("%directory%", &directory);
        body = body.replace("%parent%", &parent_directory);
        body = body.replace("%version%", &version);
        body = body.replace("%name%", &name);

        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), body.len().to_string());
        headers.insert("Content-Type".to_string(), "text/html".to_string());
        headers.insert("Connection".to_string(), "close".to_string());

        Response {
            status: 200,
            headers,
            content_size: Some(body.len() as u64),
            content: Box::new(StringReader::new(body))
        }
    }

    fn get_dir_items(path: &PathBuf) -> Vec<DirItem> {
        let mut entries = path.read_dir().unwrap()
            .map(|e| e.unwrap()).
            map(|e| {
                let name = e.file_name().to_str().unwrap().to_string();
                let kind = if e.file_type().unwrap().is_dir() { 0 } else { 1 };
                let size = e.metadata().unwrap().len();
                DirItem {
                    name,
                    kind,
                    size,
                }
            })
            .collect::<Vec<DirItem>>();
        entries.sort_by_key(|e| e.kind);
        entries
    }
}