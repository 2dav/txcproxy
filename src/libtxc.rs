use std::{
    ffi::{c_int, CStr, CString, OsStr},
    io, mem,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
};

use windows_sys::Win32::Foundation::{GetLastError, HMODULE};
use windows_sys::Win32::System::Diagnostics::Debug as dbg;
use windows_sys::Win32::System::LibraryLoader as ll;

pub type Initialize = unsafe extern "C" fn(*const u8, c_int) -> *const u8;
pub type SendCommand = unsafe extern "C" fn(*const u8) -> *const u8;
pub type FreeMemory = unsafe extern "C" fn(*const u8) -> bool;
pub type UnInitialize = unsafe extern "C" fn() -> *const u8;
pub type SetCallback = unsafe extern "C" fn(Callback) -> bool;
pub type Callback = extern "C" fn(*const u8) -> bool;

pub struct Module {
    handle: HMODULE,
    pub initialize: Initialize,
    pub send_command: SendCommand,
    pub set_callback: SetCallback,
    pub free_memory: FreeMemory,
    pub uninitialize: UnInitialize,
}

impl Drop for Module {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = (self.uninitialize)();
            ll::FreeLibrary(self.handle);
        }
    }
}

const NULL: u32 = 0;

macro_rules! last_error_or {
    ($msg:expr) => {{
        match GetLastError() {
            NULL => io::Error::new(io::ErrorKind::Other, $msg),
            error => io::Error::from_raw_os_error(error as i32),
        }
    }};
}

#[inline(never)]
unsafe fn load(wide_filename: Vec<u16>) -> Result<HMODULE, io::Error> {
    let mut prev_mode = 0;

    dbg::SetThreadErrorMode(dbg::SEM_FAILCRITICALERRORS, &mut prev_mode);

    let handle = ll::LoadLibraryExW(wide_filename.as_ptr(), NULL as _, NULL);
    let ret = if handle != NULL as _ {
        Ok(handle)
    } else {
        Err(last_error_or!("Не удалось загрузить библиотеку по неизвестной причине"))
    };

    dbg::SetThreadErrorMode(prev_mode, std::ptr::null_mut());

    ret
}

impl Module {
    pub unsafe fn load<P: AsRef<OsStr>>(path: P) -> Result<Self, io::Error> {
        {
            let wide_filename: Vec<u16> =
                path.as_ref().encode_wide().chain(Some(NULL as _)).collect();
            if ll::GetModuleHandleExW(0, wide_filename.as_ptr(), &mut 0) != NULL as _ {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Библиотека уже загружена в пространство процесса",
                ));
            }

            load(wide_filename)
        }
        .and_then(|handle| {
            macro_rules! proc_addr {
                ($p:expr) => {{
                    let addr = ll::GetProcAddress(handle, $p.as_ptr().cast());
                    if addr.is_none() {
                        return Err(last_error_or!(format!(
                            "Не удалось получить адресс функции {}",
                            $p
                        )));
                    }
                    mem::transmute(addr)
                }};
            }
            Ok(Self {
                handle,
                initialize: proc_addr!("Initialize\0"),
                send_command: proc_addr!("SendCommand\0"),
                set_callback: proc_addr!("SetCallback\0"),
                free_memory: proc_addr!("FreeMemory\0"),
                uninitialize: proc_addr!("UnInitialize\0"),
            })
        })
    }

    pub fn initialize(&self, log_dir: PathBuf, logging_level: c_int) -> Result<(), String> {
        let work_dir = CString::new(log_dir.to_string_lossy().to_string()).unwrap();
        unsafe {
            match (self.initialize)(work_dir.as_ptr() as _, logging_level) {
                p if p.is_null() => Ok(()),
                p => {
                    let msg = CStr::from_ptr(p as _).to_string_lossy().to_string();
                    (self.free_memory)(p as _);
                    Err(msg)
                }
            }
        }
    }

    pub fn set_callback(&self, callback: Callback) -> bool {
        unsafe { (self.set_callback)(callback) }
    }

    #[inline]
    pub fn send_command(&self, p: *const u8) -> *const u8 {
        debug_assert!(!p.is_null());
        unsafe { (self.send_command)(p) }
    }
}
