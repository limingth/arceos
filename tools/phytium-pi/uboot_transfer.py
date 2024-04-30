# 在phytium-pi.mk执行uboot编译模块后，将编译出的内容使用uboot传输方式发送文件到phytium-pi开发板上。

import serial
import sys
import os

class UbootTransfer:
    def __init__(self, device, baund, filePath):
        self.device = device
        self.baund = baund
        self.filePath = filePath

    def checkDeivce():
        #检查串口设备是否存在
        if not os.path.exists(device):
            print("Device {} does not exist".format(device))
            sys.exit(1)

    def transfer(self):
        # 检查串口设备是否存在
        UbootTransfer.checkDeivce()
        # 打开串口
        print("open serial device")
        ser = serial.Serial(self.device, self.baund)
        line = ser.readline().decode().strip()

        ## 检测当串口输出 'Hit any key to stop autoboot:' 模拟按下输出回车
        while True:
            
            if line:
                print(line)
            # 检测当串口输出 'Hit any key to stop autoboot:' 模拟按下输出回车
            if line == 'Hit any key to stop autoboot:':
                ser.write('\n'.encode())
                break


        #灌入命令：usb start; fatload usb 0 0x90100000 文件名; go 0x90100000
        ser.write('usb start; fatload usb 0 0x90100000 {}'.format(os.path.basename(self.filePath)).encode())
        ser.write('go 0x90100000'.encode())


    
        # 在循环中模拟终端，不断接受用户输入和串口输出，当用户输入exit退出
        while True:
            # 接受用户输入
            user_input = input()
            # 发送用户输入到串口
            ser.write(user_input.encode())
            # 接受串口输出
            line = ser.readline().decode().strip()
            # 打印串口输出
            print(line)

            # 当用户输入exit退出
            if user_input == 'exit':
                break
        
        ser.close()



# 入口函数，从命令行读取参数 例： python uboot_transfer.py /dev/ttyUSB0 115200 /path/to/file.bin
if __name__ == '__main__':
    print("--Uboot Transfer--")
    device = sys.argv[1]
    baund = int(sys.argv[2])
    filePath = sys.argv[3]
    ubootTransfer = UbootTransfer(device, baund, filePath)
    ubootTransfer.transfer()