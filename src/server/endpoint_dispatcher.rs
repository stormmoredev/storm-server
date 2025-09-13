use std::net::SocketAddr;
use crate::conf::Conf;

pub struct Dispatcher {
    endpoints: Vec<SocketAddr>,
    index: usize
}

impl Dispatcher {
    pub fn new(conf: &Conf) -> Dispatcher {
        Dispatcher {
            endpoints: conf.load_balancing_servers.clone(),
            index:  0
        }
    }

    pub fn get(&mut self) -> Option<SocketAddr> {
        let endpoint = self.endpoints.get(self.index);
        self.index += 1;
        if self.index > self.endpoints.len() - 1 {
            self.index = 0;
        }
        match endpoint {
            Some(endpoint) => Some(endpoint.clone()),
            None => None
        }
    }
}