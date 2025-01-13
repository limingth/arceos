# 飞腾派开发板帮助

## Host依赖

```shell
pip3 install pyserial
pip3 install xmodem
apt install minicom
```

## u-boot 配置

1.启动倒计时结束前按任意键进入命令行

2.执行

```shell
# 设置启动参数
setenv bootcmd ""
# 保存
saveenv
```

## 启动

host先插入串口转usb，然后执行 

```shell
make A=apps/cli PLATFORM=aarch64-phytium-pi LOG=debug chainboot
```

开发板上电，并等待固件传输完成

输入 `go 0x90100000` 加载镜像，进入ArceOS
