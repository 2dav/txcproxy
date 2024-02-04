mod handler;
mod libtxc;
mod ws2;

pub use handler::handler;

use std::{
    env, io,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
};

use ah::Context;
use anyhow as ah;

const TXC_LOG_LEVEL_DEFAULT: i32 = 1;
const TXC_PROXY_LOG_LEVEL: &str = "LOG_LEVEL";
const TXC_PROXY_FORK_ENV: &str = "__TXC_PROXY_FORK";

pub enum Role {
    Master,
    Handler,
}

pub fn current_role() -> Role {
    // 'TXC_PROXY_FORK_ENV' is set by the 'master' process.
    if env::var_os(TXC_PROXY_FORK_ENV).is_some() {
        Role::Handler
    } else {
        Role::Master
    }
}

pub fn test_load_dll(dll_path: std::path::PathBuf) -> ah::Result<libtxc::Module> {
    // Test-load the library at `dll_path` to ensure it is readable and points to the sought-after
    // dynamic library.
    unsafe { libtxc::Module::load(dll_path.clone()).map_err(Into::into) }
}

pub fn test_write_log_dir(mut dir: std::path::PathBuf) -> ah::Result<()> {
    if !dir.exists() {
        ah::bail!("Путь {dir:?} не существует");
    }
    if !dir.is_dir() {
        ah::bail!("{dir:?} не является директорией");
    }
    if dir.metadata().map_err(Into::<ah::Error>::into)?.permissions().readonly() {
        ah::bail!("{dir:?} недоступна для записи");
    }

    dir.push("temp");
    std::fs::write(dir.clone(), "temp.file")?;
    std::fs::remove_file(dir)?;

    Ok(())
}

pub fn txc_log_level() -> i32 {
    if let Ok(level) = std::env::var(TXC_PROXY_LOG_LEVEL) {
        level.parse::<i32>().unwrap_or(TXC_LOG_LEVEL_DEFAULT)
    } else {
        TXC_LOG_LEVEL_DEFAULT
    }
}

pub fn read_handler_params() -> ah::Result<(TcpStream, PathBuf, PathBuf, IpAddr)> {
    // 'master' process ensures that the handler 'fork' is run with the 'dll_path', 'log_dir' and 'addr'
    // as positional arguments, and client socket handle is written to 'stdin'

    // init winsock2
    drop(std::net::TcpListener::bind("255.255.255.255:0"));

    let con = ws2::read_pinfo(io::stdin())
        .context("Не удалось получить дескриптор подключения через 'stdin'")?;

    match env::args().collect::<Vec<String>>().as_slice() {
        [.., log_dir, dll_path, addr] => {
            Ok((con, log_dir.into(), dll_path.into(), addr.parse().expect("infallible")))
        }
        _ => unreachable!("unexpected number of arguments"),
    }
}

pub fn master<A: Into<SocketAddr>>(addr: A, log_dir: PathBuf, dll_path: PathBuf) -> ah::Result<()> {
    // Start the socket server
    let addr = addr.into();
    let listener =
        TcpListener::bind(addr).with_context(|| format!("Ошибка регистрации сокета {:?}", addr))?;

    println!("Сервер запущен на {:?}", listener.local_addr()?);

    // For each incoming connection "fork" the current process and pass the socket handle for
    // further processing.
    for conn in listener.incoming() {
        let conn = conn?;
        let mut child = Command::new(env::current_exe()?)
            .env(TXC_PROXY_FORK_ENV, "")
            .current_dir(env::current_dir()?)
            .stderr(Stdio::inherit())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .arg(dll_path.to_string_lossy().to_string())
            .arg(log_dir.to_string_lossy().to_string())
            .arg(addr.ip().to_string())
            .spawn()
            .context("Ошибка запуска обработчика клиентcкого подключения")?;

        let info = ws2::dup_socket(conn, child.id())
            .context("Ошибка создания копии сокета 'WSADuplicateSocketW'")?;

        // Pass the socket handle to child process 'stdin'
        child
            .stdin
            .as_mut()
            .ok_or_else(io::Error::last_os_error)
            .map_err(Into::into)
            .and_then(|writer| ws2::write_pinfo(writer, info))
            .context("Ошибка передачи подключения в поток обработки")?;
    }

    Ok(())
}
