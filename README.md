# Rust websocket app

## Roadmap
* Добавить поддержку OpenTelemetry
* Создание примера JS приложения чата с presence каналом.
* Добавить Ably Presence `SYNC` при подключении к каналу для получения актуального списка участников.
* Поддержка бинарных данных в запросе `POST:https://{{host}}/channels/:channel/messages`
* Сделать рефакторинг. Оптимизировать сериализацию сообщения через PreparedFrame
