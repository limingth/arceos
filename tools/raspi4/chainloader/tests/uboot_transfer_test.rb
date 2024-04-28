require 'expect'

class UBootSerialTransfer
  def initialize(serial_port, baud_rate)
    @serial_port = serial_port
    @baud_rate = baud_rate
  end

  def transfer_and_save(file_path, load_address, save_address, size)
    command = "minicom -D #{@serial_port} #{@baud_rate}"
    expect_script = <<-EXPECT_SCRIPT
            spawn #{command}
            expect "U-Boot>"
            send "loadb #{load_address}\r"
            expect "## Ready for binary (ymodem) download to #{load_address} at #{@baud_rate} bps..."
            send "\r"
            expect "Bytes received:"
            send "\x04"   # Send Ctrl+D to end transfer
            expect "U-Boot>"
            send "md #{load_address}\r"
            expect "U-Boot>"
            send "\r"
            expect "U-Boot>"
            send "save #{load_address} #{size} #{save_address}\r"
            expect "U-Boot>"
            send "\r"
            expect "U-Boot>"
            send "reset\r"
            expect "resetting ..."
            interact
    EXPECT_SCRIPT

    system("expect -c '#{expect_script}'")
  end
end

if __FILE__ == $0
  serial_transfer = UBootSerialTransfer.new("/dev/ttyUSB0", 115200)
  serial_transfer.transfer_and_save("example.bin", "0x20000000", "0x20000", "0x200000")
end