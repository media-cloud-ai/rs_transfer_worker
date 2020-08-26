use mcai_worker_sdk::start_worker;
use transfer_worker::TransferEvent;

fn main() {
  let message_event = TransferEvent::default();
  start_worker(message_event);
}
