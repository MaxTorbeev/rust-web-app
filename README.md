# Rust websocket app

## Roadmap
* Добавить поддержку OpenTelemetry
- [x] Создание примера JS приложения чата с presence каналом.
- [x] Добавить Ably Presence `SYNC` при подключении к каналу для получения актуального списка участников.
* Поддержка бинарных данных в запросе `POST:https://{{host}}/channels/:channel/messages`
* Сделать рефакторинг. Оптимизировать сериализацию сообщения через PreparedFrame
* Добавить реализацию Client events/whisper
