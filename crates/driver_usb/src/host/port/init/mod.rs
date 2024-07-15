use fully_operational::FullyOperational;
use log::debug;
use resetter::Resetter;

mod descriptor_fetcher;
mod endpoints_initializer;
pub(super) mod fully_operational;
mod max_packet_size_setter;
mod resetter;
mod slot_structures_initializer;

pub(super) fn init(port_number: u8) -> FullyOperational {
    let resetter = Resetter::new(port_number);
    debug!("reset");
    let slot_structures_initializer = resetter.reset();
    debug!("init");
    let max_packet_size_setter = slot_structures_initializer.init();
    debug!("set");
    let descriptor_fetcher = max_packet_size_setter.set();
    debug!("fetch");
    let endpoints_initializer = descriptor_fetcher.fetch();
    debug!("complete");
    endpoints_initializer.init()
}
