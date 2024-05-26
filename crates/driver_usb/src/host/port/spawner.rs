use crate::multitask;
use alloc::collections::BTreeSet;
use multitask::task::Task;
use spinning_top::Spinlock;

static SPAWN_STATUS: Spinlock<BTreeSet<usize>> = Spinlock::new(BTreeSet::new());

pub(crate) fn spawn_all_connected_ports() {
    let n = super::max_num();
    for i in 0..n {
        let _ = try_spawn(i + 1);
    }
}

pub(crate) fn try_spawn(port_number: u8) -> Result<(), PortNotConnected> {
    if spawnable(port_number) {
        spawn(port_number);
        Ok(())
    } else {
        Err(PortNotConnected)
    }
}

fn spawn(p: u8) {
    mark_as_spawned(p);
    add_task_for_port(p);
}

fn add_task_for_port(p: u8) {
    multitask::add(Task::new(super::main(p)));
}

fn spawnable(p: u8) -> bool {
    super::connected(p) && !spawned(p)
}

fn spawned(p: u8) -> bool {
    SPAWN_STATUS.lock().contains(&p.into())
}

fn mark_as_spawned(p: u8) {
    SPAWN_STATUS.lock().insert(p.into());
}

#[derive(Debug)]
pub(crate) struct PortNotConnected;
