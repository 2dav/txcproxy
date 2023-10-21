#!/bin/python
import socket
import time
from threading import Thread

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
            print("data: ", b)

Thread(target=rcv_print_loop, args=(rcv, )).start()


# 3. Send command
conn_ver = b"""<command id = \"get_connector_version\"/>\0"""
while True:
    cmd.sendall(conn_ver)
    print("resp: ", cmd.recv(256))
    time.sleep(2)


