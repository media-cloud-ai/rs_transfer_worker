use mcai_worker_sdk::prelude::start_worker;
use rs_transfer_worker::TransferEvent;

fn main() {
    let message_event = TransferEvent::default();
    start_worker(message_event);
}
