use libtxc::{LogLevel, Stream, TransaqConnector};
use std::{
    env,
    error::Error,
    io::{self, BufRead, BufReader, Read, Write},
    mem,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    os::windows::io::{FromRawSocket, IntoRawSocket, RawSocket},
    path::PathBuf,
    process::{Command, Stdio},
};
use winapi::um::winsock2::{
    closesocket, WSADuplicateSocketW, WSAGetLastError, WSASocketW, FROM_PROTOCOL_INFO,
    INVALID_SOCKET, SOCKET, WSAPROTOCOL_INFOW, WSA_FLAG_OVERLAPPED,
};

const TXC_PROXY_FORK_ENV: &str = "__TXC_PROXY_FORK";
const TXC_PROXY_LOG_LEVEL: &str = "TXC_PROXY_LOG_LEVEL";

pub type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

#[inline(always)]
fn last_os_error() -> io::Error {
    io::Error::last_os_error()
}

#[inline(always)]
fn last_ws_error() -> io::Error {
    unsafe { io::Error::from_raw_os_error(WSAGetLastError()) }
}

#[inline(always)]
fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
}

fn bind_any() -> io::Result<TcpListener> {
    for port in 1025..65535 {
        if let Ok(listener) = bind(port) {
            return Ok(listener);
        }
    }
    Err(last_os_error())
}

fn lib_path() -> io::Result<PathBuf> {
    let path = env::current_dir()?.join("txcn64.dll");
    if path.exists() {
        Ok(path)
    } else {
        let path = env::current_dir()?.join("txmlconnector64.dll");
        if path.exists() {
            Ok(path)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "library not found"))
        }
    }
}

fn log_dir(id: u16) -> Result<PathBuf> {
    let wd = env::current_dir()?;
    let log_dir = wd.join("sessions").join(id.to_string());
    std::fs::create_dir_all(log_dir.clone())?;
    Ok(log_dir)
}

fn log_level() -> LogLevel {
    match env::var(TXC_PROXY_LOG_LEVEL) {
        Ok(s) => (s.parse::<u8>().unwrap_or(1) as i32).into(),
        _ => LogLevel::Minimum,
    }
}

fn init_data_conn(stream: &mut TcpStream) -> Result<(u16, TcpStream)> {
    // open data socket
    let listener = bind_any()?;
    let data_port = listener.local_addr()?.port();
    // send data port to client
    stream.write_all(&data_port.to_le_bytes())?;
    // wait for connection
    let (ds, _) = listener.accept()?;
    ds.shutdown(std::net::Shutdown::Read)?;
    Ok((data_port, ds))
}

fn handle_conn(mut cmd_stream: TcpStream) -> Result {
    let (data_port, mut data_stream) = init_data_conn(&mut cmd_stream)?;
    let mut txc = TransaqConnector::new(lib_path()?, log_dir(data_port)?, log_level())?;
    let sender = txc.sender();
    txc.input_stream().subscribe(move |buf| {
        let _ = data_stream.write_all(buf.to_bytes());
    });

    let mut reader = BufReader::new(cmd_stream.try_clone()?);
    let mut buff = Vec::with_capacity(1 << 20);

    while !matches!(reader.read_until(b'\0', &mut buff), Ok(0) | Err(_)) {
        match unsafe { sender.send(&buff) } {
            Ok(resp) => cmd_stream.write_all(resp.to_bytes())?,
            Err(e) => cmd_stream.write_all(e.to_string().as_bytes())?,
        };
        buff.clear();
    }
    Ok(())
}

fn handler() -> Result {
    // before using any winsock2 stuff it should be initialized(WSAStartup), let libstd handle this
    drop(TcpListener::bind("255.255.255.255:0"));
    // read socket info from stdin
    let mut buff = [0u8; mem::size_of::<WSAPROTOCOL_INFOW>()];
    io::stdin().read_exact(&mut buff)?;
    // reconstruct socket
    let con = unsafe {
        match WSASocketW(
            FROM_PROTOCOL_INFO,
            FROM_PROTOCOL_INFO,
            FROM_PROTOCOL_INFO,
            &mut *(buff.as_ptr() as *mut WSAPROTOCOL_INFOW),
            0,
            WSA_FLAG_OVERLAPPED,
        ) {
            INVALID_SOCKET => Err(last_ws_error()),
            socket => Ok(TcpStream::from_raw_socket(socket as RawSocket)),
        }
    }?;
    handle_conn(con)
}

fn spawn_handler(stream: TcpStream) -> Result {
    // fork
    let mut proc = Command::new(env::current_exe()?)
        .env(TXC_PROXY_FORK_ENV, "")
        .current_dir(env::current_dir()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;
    let pid = proc.id();
    let sin = proc.stdin.as_mut().ok_or_else(last_os_error)?;

    // duplicate socket
    let raw_fd = stream.into_raw_socket();
    let pl = unsafe {
        let mut pi: WSAPROTOCOL_INFOW = mem::zeroed();
        if WSADuplicateSocketW(raw_fd as SOCKET, pid, &mut pi) != 0 {
            return Err(last_ws_error())?;
        }
        std::slice::from_raw_parts(
            mem::transmute::<_, *const u8>(&pi),
            mem::size_of::<WSAPROTOCOL_INFOW>(),
        )
    };
    // send socket info to child's stdin
    sin.write_all(pl)?;
    // finally close our copy of the socket
    unsafe { closesocket(raw_fd as SOCKET) };
    Ok(())
}

fn server() -> Result {
    let control_port = env::args().next_back().and_then(|p| p.parse().ok()).unwrap_or(5555);

    let listener = bind(control_port).or_else(|err| {
        eprintln!("127.0.0.1:{} bind error {}", control_port, err);
        bind_any()
    })?;

    println!("Сервер запущен на: {}", listener.local_addr()?.port());
    for conn in listener.incoming() {
        spawn_handler(conn?)?;
    }
    Ok(())
}

pub fn main() -> Result {
    if env::var(TXC_PROXY_FORK_ENV).is_ok() {
        env::remove_var(TXC_PROXY_FORK_ENV);
        handler()
    } else {
        server()
    }
}
