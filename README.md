txcProxy 
------------
Многопользовательский TCP/IP proxy-cервер для работы с библиотекой TRANSAQ XML Connector.

### Установка
Собранный `txcproxy.exe` актуальной версии можно скачать 
со [страницы релизов](https://github.com/2dav/txcproxy/releases/latest).

### Использование
```bash
txcproxy.exe --help
```
```
Transaq XML Connector Proxy Server

Usage: txcproxy.exe [OPTIONS]

Options:
  -d, --dll <FILE>     Путь к библиотеке "Transaq XML Connector" [default: ./txmlconnector64.dll]
  -l, --logdir <FILE>  Путь к директории для записи логов работы коннектора [default: ./sessions]
  -a, --addr <ADDR>    Адрес для входящих подключений [default: 127.0.0.1]
  -p, --port <PORT>    Порт для входящих подключений [default: 4242]
  -h, --help           Print help
  -V, --version        Print version
```

Для каждого подключения на основной порт(command port) сервер инициализирует экземпляр библиотеки, 
отправляет клиенту номер порта для приёма асинхронных сообщений коннектора(data port) и ожидает 
подключение на этом порту. Цикл приёма/отправки начинается после подключения на data port.

### Примеры
Запустите сервер
```bash
> txcproxy.exe -d <path/to/txmlconnector.dll>

Сервер запущен на 127.0.0.1:4242
```
Примеры в директории [examples](examples/) демонстрируют установку связи с сервером и 
особенности использования.
> python examples/simple.py

Базовый пример использования, после подключения к серверу отправляет не требующую авторизации команду
и ожидает ответ коннектора.

> python examples/connect.py <LOGIN> <PASSWORD>

Пример подключения к серверу Transaq(требуются логин и пароль).

### Альтернативы
- [novikovag/TXCProxy](https://github.com/novikovag/TXCProxy) `C` 
- [kmlebedev/txmlconnector](https://github.com/kmlebedev/txmlconnector) `Go` 

### License
<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>
<br/>
<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
