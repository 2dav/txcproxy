use anyhow::{self as ah, Context};
use clap::Parser;
use std::{net::IpAddr, path::PathBuf};
use txcproxy::{
    current_role, handler, master, read_handler_params, test_load_dll, test_write_log_dir, Role,
};

///Transaq XML Connector Proxy Server
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Путь к библиотеке "Transaq XML Connector"
    #[arg(short, long, value_name = "FILE", default_value = "./txmlconnector64.dll")]
    dll: PathBuf,

    /// Путь к директории для записи логов работы коннектора
    #[arg(short, long, value_name = "FILE", default_value = "./sessions")]
    logdir: PathBuf,

    /// Адрес для входящих подключений
    #[arg(short, long, default_value_t = [127, 0, 0, 1].into())]
    addr: IpAddr,

    /// Порт для входящих подключений
    #[arg(short, long, default_value_t = 4242)]
    port: u16,
}

fn main() -> ah::Result<()> {
    match current_role() {
        Role::Master => {
            let cli = Cli::parse();

            // Try to catch two common sources of errors, do it here to fail
            // early(before any client connects), in case.
            test_load_dll(cli.dll.clone()).with_context(|| {
                format!("Не удалось загрузить библиотеку {:?}", cli.dll.clone())
            })?;
            test_write_log_dir(cli.logdir.clone()).with_context(|| {
                format!("Ошибка обращения к директории {:?}", cli.logdir.clone())
            })?;

            master((cli.addr, cli.port), cli.logdir, cli.dll).context("Ошибка запуска сервера")
        }
        Role::Handler => read_handler_params()
            .context("Ошибка инициализации обработчика подключения")
            .and_then(|(con, dll_path, log_dir)| {
                handler(con, dll_path, log_dir).context("Ошибка обработки подключения")
            }),
    }
}
