use log::debug;

use crate::{types::ConfigCommand, Access, PciAddress};

#[derive(Clone)]
pub struct PhytiumPCIeDummy {}

const RGR1_SW_INIT_1: usize = 0x9210;
const EXT_CFG_INDEX: usize = 0x9000;
const EXT_CFG_DATA: usize = 0x8000;

fn cfg_index(addr: PciAddress) -> usize {
    ((addr.device as u32) << 15 | (addr.function as u32) << 12 | (addr.bus as u32) << 20) as usize
}

impl Access for PhytiumPCIeDummy {
    fn setup(mmio_base: usize) {
        debug!("PCIe link start @0x{:X}...", mmio_base);
        debug!(
            "theroticly, since uboot had already initialized it, we need't to operate it any more!"
        )
    }

    fn probe_bridge(mmio_base: usize, bridge_header: &crate::types::ConifgPciPciBridge) {
        debug!("bridge phytium weird pcie chip");

        bridge_header.set_cache_line_size(64 / 4);
        bridge_header.set_memory_base((0xF8000000u32 >> 16) as u16);
        bridge_header.set_memory_limit((0xF8000000u32 >> 16) as u16);
        bridge_header.set_control(0x01);
        unsafe {
            (bridge_header.cfg_addr as *mut u8)
                .offset(0xac + 0x1c)
                .write_volatile(0x10);
        }

        bridge_header.to_header().set_command([
            ConfigCommand::MemorySpaceEnable,
            ConfigCommand::BusMasterEnable,
            ConfigCommand::ParityErrorResponse,
            ConfigCommand::SERREnable,
        ])
    }

    fn map_conf(mmio_base: usize, addr: crate::PciAddress) -> Option<usize> {
        // // bus 0 bus 1 只有一个Device
        // if addr.bus <= 2 && addr.device > 0 {
        //     return None;
        // }

        if addr.bus == 0 {
            return Some(mmio_base);
        }

        let idx = cfg_index(addr);
        unsafe {
            ((mmio_base + EXT_CFG_INDEX) as *mut u32).write_volatile(idx as u32);
        }
        return Some(mmio_base + EXT_CFG_DATA);
    }
}
