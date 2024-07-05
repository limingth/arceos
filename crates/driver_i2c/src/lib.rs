#![no_std]
use log::debug;
const GPIO0_BASE: u32 = 0x000_2803_5000;
const GPIO_SWPORT_DR: u32 = GPIO0_BASE + 0x00;
const GPIO_SWPORT_DDR: u32 = GPIO0_BASE + 0x04;

fn write_gpio1_11_value(num: bool) {
    unsafe {
        (GPIO_SWPORT_DDR as *mut u32).write_volatile(0xFFFF);
        let current_ddr = (GPIO_SWPORT_DR as *mut u32).read_volatile();
        if (num){
            (GPIO_SWPORT_DR as *mut u32).write_volatile(current_ddr|0x0800);
        }
        else{
            (GPIO_SWPORT_DR as *mut u32).write_volatile(current_ddr|0x0000);
        }
    }
}

fn write_gpio1_12_value(num:bool) {
    unsafe {
        (GPIO_SWPORT_DDR as *mut u32).write_volatile(0xFFFF);
        let current_ddr = (GPIO_SWPORT_DR as *mut u32).read_volatile();
        if (num){
            (GPIO_SWPORT_DR as *mut u32).write_volatile(current_ddr|0x1000);
        }
        else{
            (GPIO_SWPORT_DR as *mut u32).write_volatile(current_ddr|0x0000);
        }
    }
}

fn OLED_W_SCL(num:bool){
    write_gpio1_11_value(num);
}

fn OLED_W_SDA(num:bool){
    write_gpio1_12_value(num);
}

/*引脚初始化*/
fn OLED_I2C_Init()
{
	OLED_W_SCL(true);
	OLED_W_SDA(true);
}

/**
  * @brief  I2C开始
  * @param  无
  * @retval 无
  */
fn OLED_I2C_Start()
{
	OLED_W_SDA(true);
	OLED_W_SCL(true);
	OLED_W_SDA(false);
	OLED_W_SCL(false);
}

/**
  * @brief  I2C停止
  * @param  无
  * @retval 无
  */
fn OLED_I2C_Stop()
{
	OLED_W_SDA(false);
	OLED_W_SCL(true);
	OLED_W_SDA(true);
}

/**
  * @brief  I2C发送一个字节
  * @param  Byte 要发送的一个字节
  * @retval 无
  */
fn OLED_I2C_SendByte(Byte:u8)
{
	for i in 0..8
	{
		let bit = (Byte & (0x80 >> i)) != 0;
        OLED_W_SDA(bit);
		OLED_W_SCL(true);
		OLED_W_SCL(false);
	}
	OLED_W_SCL(true);	//额外的一个时钟，不处理应答信号
	OLED_W_SCL(false);
}

/**
  * @brief  OLED写命令
  * @param  Command 要写入的命令
  * @retval 无
  */
fn OLED_WriteCommand(Command:u8)
{
	OLED_I2C_Start();
	OLED_I2C_SendByte(0x3C);		//从机地址
	OLED_I2C_SendByte(0x00);		//写命令
	OLED_I2C_SendByte(Command); 
	OLED_I2C_Stop();
}

/**
  * @brief  OLED写数据
  * @param  Data 要写入的数据
  * @retval 无
  */
fn OLED_WriteData(Data:u8)
{
	OLED_I2C_Start();
	OLED_I2C_SendByte(0x3C);		//从机地址
	OLED_I2C_SendByte(0x40);		//写数据
	OLED_I2C_SendByte(Data);
	OLED_I2C_Stop();
}

/**
  * @brief  OLED设置光标位置
  * @param  Y 以左上角为原点，向下方向的坐标，范围：0~7
  * @param  X 以左上角为原点，向右方向的坐标，范围：0~127
  * @retval 无
  */
fn OLED_SetCursor(Y:u8, X:u8)
{
	OLED_WriteCommand(0xB0 | Y);					//设置Y位置
	OLED_WriteCommand(0x10 | ((X & 0xF0) >> 4));	//设置X位置高4位
	OLED_WriteCommand(0x00 | (X & 0x0F));			//设置X位置低4位
}

/**
  * @brief  OLED清屏
  * @param  无
  * @retval 无
  */
fn OLED_Clear()
{  

	for j in 0..8
	{
		OLED_SetCursor(j, 0);
		for i in 0..128
		{
			OLED_WriteData(0x00);
		}
	}
}

fn oled_fill_screen(){
    for j in 0..8
	{
		OLED_SetCursor(j, 0);
		for i in 0..128
		{
			OLED_WriteData(0xFF);
		}
	}
}


fn OLED_Init()
{
	for i in 0..1000		
	{
		for j in 0..1000{};
	}
	
	OLED_I2C_Init();			//端口初始化
	
	OLED_WriteCommand(0xAE);	//关闭显示
	
	OLED_WriteCommand(0xD5);	//设置显示时钟分频比/振荡器频率
	OLED_WriteCommand(0x80);
	
	OLED_WriteCommand(0xA8);	//设置多路复用率
	OLED_WriteCommand(0x3F);
	
	OLED_WriteCommand(0xD3);	//设置显示偏移
	OLED_WriteCommand(0x00);
	
	OLED_WriteCommand(0x40);	//设置显示开始行
	
	OLED_WriteCommand(0xA1);	//设置左右方向，0xA1正常 0xA0左右反置
	
	OLED_WriteCommand(0xC8);	//设置上下方向，0xC8正常 0xC0上下反置

	OLED_WriteCommand(0xDA);	//设置COM引脚硬件配置
	OLED_WriteCommand(0x12);
	
	OLED_WriteCommand(0x81);	//设置对比度控制
	OLED_WriteCommand(0xCF);

	OLED_WriteCommand(0xD9);	//设置预充电周期
	OLED_WriteCommand(0xF1);

	OLED_WriteCommand(0xDB);	//设置VCOMH取消选择级别
	OLED_WriteCommand(0x30);

	OLED_WriteCommand(0xA4);	//设置整个显示打开/关闭

	OLED_WriteCommand(0xA6);	//设置正常/倒转显示

	OLED_WriteCommand(0x8D);	//设置充电泵
	OLED_WriteCommand(0x14);

	OLED_WriteCommand(0xAF);	//开启显示
		
	OLED_Clear();				//OLED清屏
}


pub fn run_iicoled() {
    OLED_Init();
    while true {
        unsafe {
            oled_fill_screen();
        }
    }
}
