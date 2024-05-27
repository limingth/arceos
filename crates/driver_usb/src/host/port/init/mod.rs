use fully_operational::FullyOperational;
use resetter::Resetter;

mod descriptor_fetcher;
mod endpoints_initializer;
pub(super) mod fully_operational;
mod max_packet_size_setter;
mod resetter;
mod slot_structures_initializer;

pub(super) fn init(port_number: u8) -> FullyOperational {
    let resetter = Resetter::new(port_number);
    let slot_structures_initializer = resetter.reset();
    let max_packet_size_setter = slot_structures_initializer.init();
    let descriptor_fetcher = max_packet_size_setter.set();
    let endpoints_initializer = descriptor_fetcher.fetch();
    endpoints_initializer.init()
}
