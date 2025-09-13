pub struct Cache {
    
}
pub struct CacheStatistic {
    pub size: i32,
}

impl CacheStatistic {
    pub fn new() -> CacheStatistic {
        CacheStatistic {
            size: 0,
        }
    }
    
    pub fn recalculate_files() -> Vec<String>{
        vec![]
    }
}