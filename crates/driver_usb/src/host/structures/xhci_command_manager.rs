use conquer_once::spin::OnceCell;
use spinning_top::Spinlock;

struct CommandManager {}

static COMMAND_MANAGER: OnceCell<Spinlock<EventManager>> = OnceCell::uninit();
