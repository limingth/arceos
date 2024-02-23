use log::info;

// 定义CommandType结构体
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    // 使能中断
    EnableInterrupt = 0b000001,
    // 禁用中断
    DisableInterrupt = 0b000010,
    // 设置端口功率
    SetPortPower = 0b000011,
    // 重置端口
    ResetPort = 0b000100,
    // 配置端点
    ConfigureEndpoint = 0b000101,
    // 评估上下文
    EvaluateContext = 0b000110,
    // 重置端点
    ResetEndpoint = 0b000111,
    // 停止端点
    StopEndpoint = 0b001000,
    // 设置端点委托状态
    SetEndpointDequeueState = 0b001001,
    // 重置设备
    ResetDevice = 0b001010,
    // 其他命令类型
    // ...
}

// 实现CommandType结构体的方法
impl CommandType {
    // 将命令类型转换为u32
    pub fn to_u32(&self) -> u32 {
        info!("to_u32");
        // 将命令类型转换为u8
        let value = (*self) as u8;
        // 将命令类型左移10位，以对齐TRB的字段
        (value as u32) << 10
    }
}

// 定义CommandTrb结构体
#[repr(C)]
pub struct CommandTrb {
    // 参数1
    parameter1: u32,
    // 参数2
    parameter2: u32,
    // 状态和控制
    status_and_control: u32,
    // 命令类型
    command_type: u32,
}

// 实现CommandTrb结构体的方法
impl CommandTrb {
    // 设置参数1
    pub fn set_parameter1(&mut self, value: u32) {
        info!("set_parameter1");
        self.parameter1 = value;
    }

    // 设置参数2
    pub fn set_parameter2(&mut self, value: u32) {
        info!("set_parameter2");
        self.parameter2 = value;
    }

    // 设置状态和控制
    pub fn set_status_and_control(&mut self, value: u32) {
        info!("set_status_and_control");
        self.status_and_control = value;
    }

    // 设置命令类型
    pub fn set_type(&mut self, value: CommandType) {
        info!("set_type");
        info!("set_type:{:x}", value.to_u32());
        self.command_type = value.to_u32();
    }

    // 设置中断目标
    pub fn set_interrupt_target(&mut self, value: u8) {
        info!("set_interrupt_target");
        self.status_and_control |= (value as u32) << 22;
    }

    // 设置循环位
    pub fn set_cycle_bit(&mut self, value: u32) {
        info!("set_cycle_bit");
        self.status_and_control |= value & 1;
    }
}
