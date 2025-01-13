
## 1. 环境和工具准备



## 2. 系统移植

Arceos在前期移植系统中的情况是启动之后什么都没输出死机，我们为了找到死机原因所在添加了`put_debug2`函数（未开启分页后输出字符）和`put_debug_paged2`函数（开启分页后输出字符）与飞腾派默认的UART1串口通信输出字符，在开机启动过程中的每个函数之间都插入这个函数，通过观察串口中输出的字符个数来判断系统启动过程中是哪个函数导致了死机。

```rust
#[cfg(all(target_arch = "aarch64", platform_family = "aarch64-phytium-pi"))]
#[no_mangle]
unsafe extern "C" fn put_debug2() {
    let state = (0x2800D018 as usize) as *mut u8;
    let put = (0x2800D000 as usize) as *mut u8;
    while (ptr::read_volatile(state) & (0x20 as u8)) != 0 {}
    *put = b'a';
}

#[cfg(all(target_arch = "aarch64", platform_family = "aarch64-phytium-pi"))]
#[no_mangle]
unsafe extern "C" fn put_debug_paged2() {
    let state = (0xFFFF00002800D018 as usize) as *mut u8;
    let put = (0xFFFF00002800D000 as usize) as *mut u8;
    while (ptr::read_volatile(state) & (0x20 as u8)) != 0 {}
    *put = b'a';
}
```

我们定位到了是`axprint!`宏导致的死机原因，随后正确的修改了串口配置。

下面是修改过的配置清单
### platforms/aarch64-phytium-pi.toml
```
kernel-base-paddr = "0x90100000"
kernel-base-vaddr = "0xffff_0000_9010_0000"
uart-paddr = "0x2800_D000"
uart-irq = "24"
```
### tools/phytium-pi/phytium-pi.its
```
load = <0x90100000>;
entry = <0x90100000>;
```


## 3.串口驱动开发应用

根据手册定义需要的寄存器（默认波特率是115200，无需定义处理波特率相关寄存器）

![[Pasted image 20240714095350.png]]

```rust
register_structs! {
    /// Pl011 registers.
    Pl011UartRegs {
        /// Data Register.
        (0x00 => dr: ReadWrite<u32>),
        (0x04 => _reserved0),
        /// Flag Register.
        (0x18 => fr: ReadOnly<u32>),
        (0x1c => _reserved1),
        /// Control register.
        (0x30 => cr: ReadWrite<u32>),
        /// Interrupt FIFO Level Select Register.
        (0x34 => ifls: ReadWrite<u32>),
        /// Interrupt Mask Set Clear Register.
        (0x38 => imsc: ReadWrite<u32>),
        /// Raw Interrupt Status Register.
        (0x3c => ris: ReadOnly<u32>),
        /// Masked Interrupt Status Register.
        (0x40 => mis: ReadOnly<u32>),
        /// Interrupt Clear Register.
        (0x44 => icr: WriteOnly<u32>),
        (0x48 => @END),
    }
}
```

实现初始化，读写字符，响应中断

```rust
/// The Pl011 Uart
///
/// The Pl011 Uart provides a programing interface for:
/// 1. Construct a new Pl011 UART instance
/// 2. Initialize the Pl011 UART
/// 3. Read a char from the UART
/// 4. Write a char to the UART
/// 5. Handle a UART IRQ
pub struct Pl011Uart {
    base: NonNull<Pl011UartRegs>,
}

unsafe impl Send for Pl011Uart {}
unsafe impl Sync for Pl011Uart {}

impl Pl011Uart {
    /// Constrcut a new Pl011 UART instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &Pl011UartRegs {
        unsafe { self.base.as_ref() }
    }

    /// Initializes the Pl011 UART.
    ///
    /// It clears all irqs, sets fifo trigger level, enables rx interrupt, enables receives
    pub fn init(&mut self) {
        // clear all irqs
        self.regs().icr.set(0x7ff);

        // set fifo trigger level
        self.regs().ifls.set(0); // 1/8 rxfifo, 1/8 txfifo.

        // enable rx interrupt
        self.regs().imsc.set(1 << 4); // rxim

        // enable receive
        self.regs().cr.set((1 << 0) | (1 << 8) | (1 << 9)); // tx enable, rx enable, uart enable
    }

    /// Output a char c to data register
    pub fn putchar(&mut self, c: u8) {
        while self.regs().fr.get() & (1 << 5) != 0 {}
        self.regs().dr.set(c as u32);
    }

    /// Return a byte if pl011 has received, or it will return `None`.
    pub fn getchar(&mut self) -> Option<u8> {
        if self.regs().fr.get() & (1 << 4) == 0 {
            Some(self.regs().dr.get() as u8)
        } else {
            None
        }
    }

    /// Return true if pl011 has received an interrupt
    pub fn is_receive_interrupt(&self) -> bool {
        let pending = self.regs().mis.get();
        pending & (1 << 4) != 0
    }

    /// Clear all interrupts
    pub fn ack_interrupts(&mut self) {
        self.regs().icr.set(0x7ff);
    }
}

```
## 4.USB驱动应用开发（高级拓展功能）




