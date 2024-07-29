#[derive(Debug, Clone)]
pub struct InterruptTransfer {
    endpoint_id: usize,
    buffer_addr_len: (usize, usize),
}
