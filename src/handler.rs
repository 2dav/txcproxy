use std::{
    ffi,
    io::{self, BufRead, Write},
    mem, net,
    path::PathBuf,
};

use super::{libtxc, ws2};
use ah::Context;
use anyhow as ah;

static mut UPSTREAM: mem::MaybeUninit<net::TcpStream> = mem::MaybeUninit::uninit();
static mut TXC_FREE_MEMORY: mem::MaybeUninit<libtxc::FreeMemory> = mem::MaybeUninit::uninit();

extern "C" fn txc_callback(buf: *const u8) -> bool {
    unsafe { UPSTREAM.assume_init_mut().write_all(TxcStr::new(buf).as_slice()).is_ok() }
}

fn init_txc(dll_path: PathBuf, log_dir: PathBuf) -> ah::Result<libtxc::Module> {
    let lib = unsafe { libtxc::Module::load(dll_path) }?;
    if let Err(msg) = lib.initialize(log_dir, super::txc_log_level()) {
        ah::bail!(msg);
    }
    lib.set_callback(txc_callback);
    unsafe { TXC_FREE_MEMORY.write(lib.free_memory) };

    Ok(lib)
}

fn init_upstream(con: &mut net::TcpStream) -> ah::Result<()> {
    // open 'data' server
    let (data_server, data_port) =
        ws2::bind_any().context("Не удалось зарегистрировать сервер трансляции данных")?;

    // send 'data' server port to client and await for connection
    con.write_all(&data_port.to_ne_bytes())?;

    let (tx, _) = data_server.accept()?;
    tx.shutdown(net::Shutdown::Read)?;
    unsafe { UPSTREAM.write(tx) };

    Ok(())
}

pub fn handler(mut con: net::TcpStream, dll_path: PathBuf, log_dir: PathBuf) -> ah::Result<()> {
    init_upstream(&mut con)?;
    let lib = init_txc(dll_path, log_dir)?;

    // loop
    let mut reader = io::BufReader::new(con.try_clone()?);
    let mut buf = Vec::with_capacity(4 << 10);

    // TODO: fix double buffering
    while !matches!(reader.read_until(b'\0', &mut buf), Ok(0) | Err(_)) {
        let resp = lib.send_command(buf.as_ptr());
        con.write_all(unsafe { TxcStr::new(resp).as_slice() })?;

        buf.clear();
    }

    Ok(())
}

struct TxcStr(*const u8);

impl TxcStr {
    pub unsafe fn new(p: *const u8) -> Self {
        Self(p)
    }

    pub fn as_slice(&self) -> &[u8] {
        extern "C" {
            /// Provided by libc or compiler_builtins.
            fn strlen(s: *const ffi::c_char) -> usize;
        }

        unsafe { std::slice::from_raw_parts(self.0, strlen(self.0 as _) + 1) }
    }
}

impl Drop for TxcStr {
    fn drop(&mut self) {
        unsafe { (TXC_FREE_MEMORY.assume_init())(self.0) };
    }
}
