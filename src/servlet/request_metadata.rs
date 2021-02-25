use std::time::{Duration, SystemTime};

pub(crate) struct RequestMetadata {
    pub id: uuid::Uuid,
    pub request_start: u64,
    pub response_end: u64,
    pub response_packet_count: i32,
}

impl RequestMetadata {
    pub fn new() -> RequestMetadata {
        RequestMetadata {
            id: uuid::Uuid::new_v4(),
            request_start: 0,
            response_end: 0,
            response_packet_count: 0,
        }
    }
    pub fn tag_request_start_time(&mut self) {
        self.request_start = RequestMetadata::millis_since_epoch();
    }
    pub fn tag_response_end_time(&mut self) {
        self.response_end = RequestMetadata::millis_since_epoch();
    }
    pub fn get_request_response_duration(&self) -> f32 {
        (self.response_end - self.request_start) as f32 / 1_000_000.0
    }
    fn millis_since_epoch() -> u64 {
        let since_the_epoch: Duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("get millis error");
        (since_the_epoch.as_secs() * 1_000_000) + (since_the_epoch.subsec_nanos() as u64)
    }
}