* 重新组织代码，使xhci控制器可以构造出多个实例（目前各种杂七杂八都是单例模式）

# 待实现结构(来自circle)
注：其中似乎有一部分结构xhci crate中有
注：若要实现其中一个，则应将其包含的所有结构也实现
注：正在做的部分后面加上自己的id
[参考xhciRootPort的启动流程](xhci_root_port_init_graph.svg)

# XHCI 部分
* ~~xhciroothub~~
* xhcirootport
* ~~xhcislotmanager~~
* ~~xhcieventmanager~~
* ~~xhcicommandmanager~~
* ~~scratchpad~~
* xhciendpoint
## 链接部分:
* xhciusbdevice 注：xhci crate中有，但是没有对应的方法，详见


# USB 部分
* usbconfigparser
* usbdevice 
* usbdevicefactory
* usbendpoint
* usbfunction
* usbhiddevice
* usbhostcontroller
* usbrequest
* usbserial
* usbsubsystem
* usbstandardhub
* usbstring
