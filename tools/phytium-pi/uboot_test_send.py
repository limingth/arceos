import sys
import serial

class UbootTransfer:
    def __init__(self, device, baud, filePath):
        self.device = device
        self.baud = baud
        self.filePath = filePath
        # 打开串口
        print("Open serial device")
        self.ser = serial.Serial(device, baud, timeout=1)


    def mainLoop(self):
        try:
            #启动后向serial发送help
            self.ser.write(b"\n")
            # 读取串口返回的数据
            response = self.ser.readline().decode().strip()
            print("Response:", response)
                

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
    ubootTransfer.mainLoop()
