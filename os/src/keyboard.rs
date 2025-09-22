use lazy_static::lazy_static;
use spin::Mutex;
use heapless::mpsc::{Queue, Producer, Consumer};

static mut QUEUE_STORAGE: Queue<u8, 16> = Queue::new();
lazy_static! {
    static ref SCANCODE_QUEUE: Mutex<(Producer<u8, 16>, Consumer<u8, 16>)> = {
        let (producer, consumer) = unsafe { QUEUE_STORAGE.split() };
        Mutex::new((producer, consumer))
    };
}

pub fn init() {
    // Keyboard initialization
}

pub fn add_scancode(scancode: u8) {
    let mut queue = SCANCODE_QUEUE.lock();
    let _ = queue.0.enqueue(scancode);
}

pub fn read_scancode() -> Option<u8> {
    let mut queue = SCANCODE_QUEUE.lock();
    queue.1.dequeue()
}