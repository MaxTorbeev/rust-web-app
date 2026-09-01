# Rust websocket app

## Документация

- [Архитектура серверов и deployment](./docs/deployment.md)
- [Health endpoints и deployment semaphore](./docs/health.md)
- [Автономный и кластерный режимы](./docs/clustering.md)

## Roadmap
- [ ] Доработать JWT
- [ ] Добавить поддержку OpenTelemetry
- [x] Создание примера JS приложения чата с presence каналом.
- [x] Добавить Ably Presence `SYNC` при подключении к каналу для получения актуального списка участников.
- [ ] Поддержка бинарных данных в запросе `POST:https://{{host}}/channels/:channel/messages`
- [x] Сделать рефакторинг. Оптимизировать сериализацию сообщения через PreparedFrame
- [ ] Добавить реализацию Client events/whisper
