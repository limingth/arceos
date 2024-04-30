phytium: build
	gzip -9 -cvf $(OUT_BIN) > arceos-phytium-pi.bin.gz
	mkimage -f tools/phytium-pi/phytium-pi.its arceos-phytiym-pi.itb
	@echo 'Built the FIT-uImage arceos-phytium-pi.itb'

chainboot:
	python tools/phytium-pi/uboot_transfer.py /dev/ttyUSB0 115200 $(OUT_BIN)
	
