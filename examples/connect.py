#!/bin/python
import socket
import sys
from threading import Thread

if len(sys.argv) < 3:
    print("Run as \npython connect.py <LOGIN> <PASSWORD>")
    sys.exit(0)

addr = socket.gethostbyname("127.0.0.1")
port = 4242

# 0. Connect to the server
print("connecting to %s:%x" % (addr, port))
cmd = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
cmd.connect((addr, port))

# 1. Receive 'data port' 
r = cmd.recv(32)
dp = int.from_bytes(r, "little")
print("< ", dp)

# 2. Connect to 'data port'
print("connecting to %s:%x" % (addr, dp))
rcv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
rcv.connect((addr, dp))

def rcv_print_loop(rcv):
    while True:
        b = rcv.recv(1 << 20).decode()
        if len(b) == 0:
            break
        else:
            print("rx: ", b)

Thread(target=rcv_print_loop, args=(rcv, )).start()


# 3. Send command
login, password = sys.argv[-2:]

connect_cmd = """<command id="connect">
  <login>%s</login>
  <password>%s</password>
  <host>tr1.finam.ru</host>
  <port>3900</port>
</command>\0""" % (login, password)

cmd.sendall(connect_cmd.encode('utf-8'))
print("< ", cmd.recv(256))

