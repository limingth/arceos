use alloc::collections::BTreeSet;
use log::debug;
use spinning_top::Spinlock;
use tock_registers::registers;

static SPAWN_STATUS: Spinlock<BTreeSet<usize>> = Spinlock::new(BTreeSet::new());

pub(crate) fn spawn_all_connected_ports() {
    let n = super::max_num();
    debug!("port numbers: {n}");
    for i in 0..n {
        let _ = try_spawn(i + 1);
    }
    debug!("done");
}

pub(crate) fn try_spawn(port_number: u8) -> Result<(), PortNotConnected> {
    if spawnable(port_number) {
        debug!("spawn port {port_number}");
        spawn(port_number);
        Ok(())
    } else {
        // Err(PortNotConnected)
        debug!("return!");
        Ok(())
    }
}

fn spawn(p: u8) {
    debug!("mark as spawned");
    mark_as_spawned(p);
    debug!("add task!");
    add_task_for_port(p);
}

fn add_task_for_port(p: u8) {
    super::main(p);
}

fn spawnable(p: u8) -> bool {
    let port = super::connected(p) && !spawned(p);
    debug!("port {p} is {port}");
    super::dump_port_status(p);
    port
}

fn spawned(p: u8) -> bool {
    SPAWN_STATUS.lock().contains(&p.into())
}

fn mark_as_spawned(p: u8) {
    SPAWN_STATUS.lock().insert(p.into());
}

#[derive(Debug)]
pub(crate) struct PortNotConnected;
