use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};

use crate::sensors;

const CAPACITY: usize = 4;
const PUBLISHERS: usize = 1;
const SUBSCRIBERS: usize = 1;
pub static TEMP_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    sensors::TempMessage,
    CAPACITY,
    SUBSCRIBERS,
    PUBLISHERS,
> = PubSubChannel::new();
