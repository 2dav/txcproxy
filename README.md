# txcProxy

Многопользовательский TCP/IP proxy-cервер для работы с библиотекой [TRANSAQ XML Connector](https://www.finam.ru/howtotrade/tconnector/) в *nix/wine.

### Сборка
##### Кросс-компиляция и запуск под wine
Установка необходимых toolchain.
```bash
rustup target add x86_64-pc-windows-gnu
rustup target add i686-pc-windows-gnu
```
Сборка
```bash
cargo build --release --target x86_64-pc-windows-gnu
# или
make 64
```
```bash
wine target/x86_64-pc-windows-gnu/release/txcproxy.exe
```
### Пример
- Отредактируйте example_client.py, введите свои логин и пароль.
- Скопируйте txmlconnect(64).dll в директорию с txcproxy.exe
- Запустите сервер
```bash
wine txcproxy.exe
```
- Запустите клиент
```bash
python example_client.py
```
### Описание
Команда запуска `wine txcproxy.exe [PORT]`. Значение `PORT` по-умолчанию 5555.

Для каждого подключения на основной порт(`command port`) сервер инициализирует экземпляр библиотеки, отправляет
клиенту номер порта для приёма асинхронных сообщений коннектора(`data port`) и ожидает
подключение на этом порту. Цикл приёма/отправки начинается после подключения на `data port`.

Данные, поступившие на `command port` передаются в команду коннектора `send_command()`, ответ коннектора передаётся клиенту на `command port`.
- сообщения должны заканчиваться `\0` байтом
- aсинхронные сообщения коннектора передаются на `data port` без завершающего `\0`
- отключение от любого из портов приводит к отключению и остановке коннектора
- логи коннектора сохраняются в ./sessions/[dataport]
- уровень логирования коннектора может быть изменён переменной
  окружения `TXC_PROXY_LOG_LEVEL` [1,2,3], 1 по-умолчанию

См. также прокси-сервер на `C` [TXCProxy](https://github.com/novikovag/TXCProxy).
