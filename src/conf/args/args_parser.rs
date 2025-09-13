use std::collections::HashMap;
use std::error::Error;

pub enum ArgKind {
    Flag(String),
    Value(String),
    Values(String)
}

pub struct ArgsParser {
    defs: Vec<ArgKind>,
}

impl ArgsParser {
    pub fn new() -> ArgsParser {
        ArgsParser {
            defs: Vec::new(),
        }
    }

    pub fn add(&mut self, kind: ArgKind) {
        self.defs.push(kind);
    }

    pub fn parse(&mut self, args: &Vec<String>) -> Result<HashMap<String, String>, Box<dyn Error>> {
        let mut storage: HashMap<String, String> = HashMap::new();

        let mut idx = 1;
        loop {
            if idx > args.len() - 1 {
                break;
            }
            let name = &args[idx];
            let kind = self.get_arg_kind(name);
            if let None = kind {
                return Err(format!("Unknown argument '{}'", name))?;
            }
            
            if let Some(kind) = kind {
                if let ArgKind::Value(_) = kind {
                    idx += 1;
                    let val = args.get(idx);
                    if let Some(val) = val {
                        storage.insert(String::from(name), String::from(val));
                    }
                }
            }
            
            idx += 1;
        }

        Ok(storage)
    }

    fn get_arg_kind(&self, name: &String) -> Option<ArgKind> {
        let defs = &self.defs;
        for def in defs {
            match def {
                ArgKind::Flag(flag) => {
                    if flag == name {
                        return Some(ArgKind::Flag(flag.to_string()));
                    }
                }
                ArgKind::Value(flag) => {
                    if flag == name {
                        return Some(ArgKind::Value(flag.to_string()));
                    }
                }
                ArgKind::Values(flag) => {
                    if flag == name {
                        return Some(ArgKind::Values(flag.to_string()));
                    }
                }
            }
        }
        None
    }
}