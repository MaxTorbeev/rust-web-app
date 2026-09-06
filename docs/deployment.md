# Архитектура серверов и deployment

## Принятое решение

На текущем этапе приложение разворачивается на одном сервере с одним публичным
IPv4-адресом.

Публичный адрес используется только для входящего HTTP/HTTPS-трафика. Rust,
Redis, NATS JetStream и OpenTelemetry Collector не публикуются напрямую в
интернет.

При переходе к отказоустойчивому кластеру клиентская точка входа также остаётся
одной: публичный IP управляемого балансировщика нагрузки. Серверы приложения,
Redis и NATS получают только приватные адреса.

| Вариант | Публичных IPv4 | Назначение |
|---|---:|---|
| Текущий single-host | 1 | HTTPS, API и WebSocket |
| HA с managed load balancer | 1 | Публичный адрес балансировщика |
| HA с managed load balancer и bastion | 2 | Клиентский трафик и административный доступ |
| Два собственных edge-сервера без floating IP | 2 | По адресу на каждый edge-сервер |

Публичные IP не назначаются отдельно каждому backend-серверу. Если для
deployment используется self-hosted runner, VPN или приватная сеть провайдера,
отдельный публичный bastion не требуется.

## Текущая single-host топология

Текущий `compose.yml` запускает на одном сервере:

- `app` — Rust HTTP/WebSocket-сервер;
- `nats` — одиночный NATS с включённым JetStream;
- `redis` — Redis с AOF;
- `otel-collector` — локальный OpenTelemetry Collector с `debug` exporter.

Системный Nginx работает вне Compose, завершает TLS, отдаёт собранный frontend и
проксирует API/WebSocket-запросы в Rust.

```text
Internet
   |
   | public IPv4, TCP 80/443
   v
system Nginx
   |-- /chat/  -> static frontend
   `-- API/WS  -> 127.0.0.1:4008
                         |
                         v
                     Rust app
                      |     |
                      |     `-- NATS JetStream
                      `-------- Redis
```

Контейнер `app` опубликован на host только как `127.0.0.1:4008`. Redis, NATS и
OTel Collector доступны только внутри Docker network.

### Сетевые порты

| Порт | Сервис | Доступ |
|---:|---|---|
| 80 | Nginx HTTP/ACME и redirect на HTTPS | Публичный |
| 443 | Nginx HTTPS/WebSocket | Публичный |
| 4008 | Rust HTTP/WebSocket | Только loopback или приватная сеть |
| 6379 | Redis | Только приватная сеть |
| 4222 | NATS client connections | Только приватная сеть |
| 6222 | NATS cluster routes, в текущем Compose не используется | Только приватная сеть |
| 8222 | NATS monitoring | Только административная приватная сеть |
| 4317 | OTLP gRPC | Только приватная сеть |
| 4318 | OTLP HTTP | Только приватная сеть |

На публичном firewall открываются только `80/tcp` и `443/tcp`. SSH открывается
только через VPN, bastion или ограниченный список доверенных адресов.

## Текущий процесс deployment

GitHub Actions выполняет deployment на один `DEPLOY_HOST`:

1. Проверяет frontend и Rust workspace.
2. Один раз собирает Docker image с тегом полного commit SHA.
3. Публикует image в GHCR.
4. Загружает на сервер `compose.yml`, `.env`, frontend artifact и deployment
   scripts.
5. При первом запуске создаёт стабильный `APP_NODE_ID` в `node.env`;
   существующий файл только проверяет и никогда не перезаписывает.
6. Выполняет `docker compose pull` и `docker compose up`.
7. Атомарно переключает symlink `frontend/current` на новый frontend release.

`APP_NODE_ID` нельзя менять при обычном перезапуске или deployment одной и
той же realtime-ноды. Он используется как имя durable consumer и как область
consumer-side дедупликации.

### Поведение при обновлении

Single-host deployment пересоздаёт контейнер приложения и разрывает активные
WebSocket-соединения. Клиенты должны переподключиться автоматически.

На текущем этапе это принимаем как ограничение single-host режима. Запуск двух
контейнеров приложения с одним и тем же `APP_NODE_ID` для blue-green
deployment недопустим: оба процесса разделят один durable consumer, и события
могут попасть только клиентам одного процесса.

## Что необходимо завершить для single-host production

- автоматизировать установку и конфигурацию Nginx;
- автоматизировать получение и обновление TLS-сертификата;
- настроить host/cloud firewall;
- добавить healthcheck приложения в Compose на основе существующего
  `/health/ready`;
- использовать readiness как deployment gate до переключения трафика;
- выполнить live smoke test полного пути:
  `publish -> JetStream -> consumer -> Redis dedup -> handler -> ACK`;
- настроить внешний exporter для OTel Collector вместо одного `debug` exporter;
- проверить backup и восстановление Redis AOF и JetStream volume.

`/health/live` и `/health/ready` уже реализованы; readiness проверяет Redis,
JetStream topology и consumer lifecycle. Но у `app` пока нет Compose
healthcheck, поэтому `docker compose up --wait` не использует этот сигнал для
приложения и сам по себе не доказывает готовность принимать трафик.

Целевой HTTP-контракт, release identity и правила допуска нод к переключению
трафика описаны в документе [Health endpoints и deployment semaphore](./health.md).

## Целевая HA-топология

Экономичный начальный вариант состоит из трёх стабильных серверов в независимых
failure domains, managed load balancer и общего HA Redis.

```text
                          Internet
                              |
                              | one public IPv4
                              v
                    Managed Load Balancer
                       /       |       \
                      /        |        \
               private     private     private
                 app-1        app-2       app-3
                 NATS-1       NATS-2      NATS-3
                      \        |        /
                       `-- private network --'
                              |
                         HA Redis
```

На первом этапе приложение и один NATS peer можно разместить на каждой из трёх
машин. При этом app и NATS запускаются раздельными Compose-проектами, имеют
раздельный lifecycle и не перезапускаются вместе.

По мере роста эластичные app-ноды можно отделить от стабильных NATS-нод. NATS
сохраняет локальные диски и стабильные server identities; приложение можно
масштабировать независимо.

### NATS JetStream

Рабочий JetStream-кластер состоит из нечётного числа NATS-серверов, обычно из
трёх:

- одинаковое имя кластера на всех серверах;
- уникальное имя каждого NATS-сервера;
- `4222/tcp` для клиентов только в приватной сети;
- `6222/tcp` для cluster routes только между NATS-серверами;
- отдельный локальный диск для каждого сервера;
- `NATS_STREAM_REPLICAS=3`;
- несколько seed routes, чтобы запуск не зависел от одного peer;
- NATS credentials, TLS и subject ACL.

Три отдельно запущенных копии текущего `compose.yml` не образуют NATS-кластер:
каждая создаст собственный standalone NATS и отдельный JetStream volume.

### Redis

Все app-ноды должны использовать один логический Redis endpoint. Redis хранит
как минимум auth sessions и EventBus consumer deduplication.

Три локальных Redis из трёх независимых Compose stack приведут к разным сессиям
и разным dedup records на app-нодах. Для HA используется managed Redis либо
отдельно спроектированная Redis replication/failover топология.

### Presence

Обычные channel messages уже имеют delivery class `AllNodes`. Presence пока
остаётся process-local, поэтому multi-node snapshot неполон; sticky sessions
этого не исправляют.

Целевые Redis state, leases/fencing, outbox, ACK/retry/dedup, snapshot barrier и
Ably-compatible Occupancy описаны в отдельном source of truth:
[Кластерный Presence и Ably-compatible Occupancy](./presence-occupancy.md).

## Rolling deployment в HA-режиме

Один Docker image собирается один раз и затем последовательно устанавливается на
все app-ноды:

1. Вывести одну ноду из load balancer.
2. Прекратить назначать ей новые соединения.
3. Дождаться завершения текущих WebSocket-соединений до установленного drain
   timeout.
4. Обновить контейнер до нового image SHA.
5. Дождаться успешного readiness:
   - HTTP-сервер принимает запросы;
   - Redis доступен;
   - NATS connection установлено;
   - JetStream stream и consumer совместимы с конфигурацией;
   - incoming consumer loop работает.
6. Вернуть ноду в load balancer.
7. Повторить процесс для следующей ноды.

При ошибке нода не возвращается в балансировщик, а приложение откатывается на
предыдущий проверенный image SHA.

Точный version-aware gate для rolling и blue-green deployment описан в
[Health endpoints и deployment semaphore](./health.md).

NATS и Redis не входят в обычный app deployment. NATS-серверы обслуживаются
отдельно и обновляются строго по одному, чтобы сохранялся quorum.

## Этапы перехода

### Этап 1. Single-host

- один сервер;
- один публичный IPv4;
- system Nginx;
- текущий Compose stack;
- допустимый reconnect клиентов при deployment.

### Этап 2. Подготовка к кластеру

- Compose healthcheck и readiness deployment gate;
- автоматизированный TLS/edge provisioning;
- live NATS/Redis smoke tests;
- Redis-backed PresenceStore с lease/TTL;
- разделение app, NATS и Redis deployment lifecycle.

### Этап 3. HA

- один managed load balancer с публичным IPv4;
- не менее двух app-нод, на старте удобно три;
- общий HA Redis;
- три стабильных NATS JetStream peer с `replicas=3`;
- rolling deployment и connection draining;
- мониторинг, backup и проверенные failure scenarios.

## Итог

Сейчас резервируется один публичный IPv4 и приватная сеть. Отдельные публичные
адреса для Rust, Redis и NATS не нужны.

Второй публичный IP появляется только при необходимости отдельного bastion или
собственного второго edge-сервера. Наличие нескольких backend-серверов само по
себе не увеличивает количество публичных клиентских адресов.

## Ссылки

- [Текущий Compose](../compose.yml)
- [Health endpoints и deployment semaphore](./health.md)
- [Целевая кластерная архитектура](./clustering.md)
- [Кластерный Presence и Ably-compatible Occupancy](./presence-occupancy.md)
- [NATS: forming a cluster](https://docs.nats.io/learn/clustering/forming-a-cluster)
- [NATS: JetStream in a cluster](https://docs.nats.io/learn/topologies/jetstream-in-a-cluster)
