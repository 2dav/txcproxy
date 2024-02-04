use std::{
    io::{self, Read, Write},
    mem, net,
    os::windows::io::{FromRawSocket, IntoRawSocket, RawSocket},
    slice,
};

use anyhow as ah;
use windows_sys::Win32::Networking::WinSock::{
    closesocket, WSADuplicateSocketW, WSAGetLastError, WSASocketW, FROM_PROTOCOL_INFO,
    INVALID_SOCKET, SOCKET, WSAPROTOCOL_INFOW, WSA_FLAG_OVERLAPPED,
};

pub fn dup_socket(stream: net::TcpStream, pid: u32) -> ah::Result<WSAPROTOCOL_INFOW> {
    let raw_fd = stream.into_raw_socket() as SOCKET;
    unsafe {
        let mut pi: WSAPROTOCOL_INFOW = mem::zeroed();
        let err = WSADuplicateSocketW(raw_fd, pid, &mut pi);
        if err != 0 {
            Err(last_ws_error().into())
        } else {
            closesocket(raw_fd);
            Ok(pi)
        }
    }
}

pub fn read_pinfo<R: Read>(mut reader: R) -> ah::Result<net::TcpStream> {
    let mut buff = [0u8; mem::size_of::<WSAPROTOCOL_INFOW>()];
    reader.read_exact(&mut buff)?;

    // "reconstruct" std::net::TcpStream from socket info
    unsafe {
        match WSASocketW(
            FROM_PROTOCOL_INFO,
            FROM_PROTOCOL_INFO,
            FROM_PROTOCOL_INFO,
            buff.as_ptr() as *const _,
            0,
            WSA_FLAG_OVERLAPPED,
        ) {
            INVALID_SOCKET => Err(last_ws_error().into()),
            socket => Ok(net::TcpStream::from_raw_socket(socket as RawSocket)),
        }
    }
}

pub fn write_pinfo<W: Write>(mut writer: W, info: WSAPROTOCOL_INFOW) -> ah::Result<()> {
    unsafe {
        let info = &info as *const _ as _;
        let info = slice::from_raw_parts(info, mem::size_of::<WSAPROTOCOL_INFOW>());
        writer.write_all(info).map_err(Into::into)
    }
}

pub fn bind_any<A: Into<Option<net::IpAddr>>>(addr: A) -> io::Result<(net::TcpListener, u16)> {
    let addr = addr.into().unwrap_or([127, 0, 0, 1].into());
    for port in 4243..65535 {
        if let Ok(listener) = net::TcpListener::bind(net::SocketAddr::new(addr, port)) {
            return Ok((listener, port));
        }
    }
    Err(io::Error::last_os_error())
}

unsafe fn last_ws_error() -> io::Error {
    io::Error::from_raw_os_error(WSAGetLastError())
}
