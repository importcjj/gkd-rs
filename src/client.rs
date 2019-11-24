

pub struct Client {
    pub peer_id: u32,
}

impl Client {
    pub fn new() -> Self {
        Self {
            peer_id: 0,
        }
    }
}