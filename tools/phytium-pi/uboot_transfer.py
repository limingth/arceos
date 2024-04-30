import serial
import sys
import os

class UbootTransfer:
    def __init__(self, device, baud, filePath):
        self.device = device
        self.baud = baud
        self.filePath = filePath
        # 打开串口
        print("Open serial device")
        self.ser = serial.Serial(device, baud, timeout=1)

    def checkDevice(self):
        # 检查串口设备是否存在
        if not os.path.exists(self.device):
            print("Device {} does not exist".format(self.device))
            sys.exit(1)

    def sendCommand(self, command): 
        self.ser.write(bytes(command+"\1a", encoding='ascii'))

    def transfer(self):
        # 检查串口设备是否存在
        self.checkDevice()

        try:
            # 等待串口输出 'Hit any key to stop autoboot:'，然后模拟输入
            # while True:
            #     line = self.ser.readline().decode().strip()
            #     if line:
            #         print(line)
            #     if 'Hit any key' in line:
            #         print("find the line: Hit any key to stop autoboot:")
            #         #self.sendCommand();
            #         self.ser.write(b'qwertyuiop\n')
            #         self.ser.write(b'asdfghjkl\n')
            #         self.ser.write(b'zxcvbnm\n')
            #         break
            

            # 他妈的我是真的服了啊，浪费将近一天的时间研究到底是怎么进入系统的
            # 一开始以为是发送的回车是有问题的，后面测试发送其他字符也没有问题
            # 手动进入中断后发送回车也是没有问题的，但是就是在等待'Hit any key to stop autoboot:'的时候不行了
            # 直接暴力做法在进入之前循环发送回车
            # 真的是太坐牢了啊
            # 在没有检测到'Phytium-Pi#'之前，循环发送回车
            while True:
                self.ser.write(b'\n')
                line = self.ser.readline().decode().strip()
                if line:
                    print(line)
                if 'Phytium-Pi#' in line:
                    print("find the line: Phytium-Pi#")
                    break

            # 检测到输出'Phytium-Pi#'字样后，模拟输入指令
            while True:
                line = self.ser.readline().decode().strip()
                if line:
                    print(line)
                if 'Phytium-Pi#' in line:
                    print("find the line: Phytium-Pi#")
                    # 发送命令：usb start; fatload usb 0 0x90100000 文件名; go 0x90100000
                    self.ser.write('usb start; fatload usb 0 0x90100000 {}\n'.format(os.path.basename(self.filePath)).encode())
                    self.ser.write(b'go 0x90100000\n')
                    break

            # 模拟终端，接收用户输入并发送到串口，同时打印串口输出
            while True:
                user_input = input()
                self.ser.write(user_input.encode() + b'\n')
                line = self.ser.readline().decode().strip()
                print(line)
                if user_input == 'exit':
                    break

        except serial.SerialException as e:
            print("Serial error:", e)

        finally:
            self.ser.close()

# 入口函数
if __name__ == '__main__':
    print("-- Uboot Transfer --")
    if len(sys.argv) != 4:
        print("Usage: python uboot_transfer.py <device> <baud> <file_path>")
        sys.exit(1)

    device = sys.argv[1]
    baud = int(sys.argv[2])
    filePath = sys.argv[3]

    ubootTransfer = UbootTransfer(device, baud, filePath)
    ubootTransfer.transfer()
