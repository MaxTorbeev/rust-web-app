# Health endpoints и deployment semaphore

## Статус документа

Этот документ фиксирует целевой контракт health endpoints и правила допуска
релиза к переключению трафика.

Контракт ещё не реализован полностью. Сейчас `src/app/http/routes/health.rs`
содержит пустой handler, маршрут не зарегистрирован, у контейнера `app` нет
healthcheck, а Git revision не вшивается в Rust binary.

Документ использует термин **deployment semaphore** или **deployment gate** для
логического условия допуска новой группы нод к трафику. Это не mutex внутри
приложения и не замена GitHub Actions `concurrency`.

## Цели

- отличать работоспособность процесса от готовности принимать новый трафик;
- видеть версию и точную Git revision каждой ноды;
- однозначно отличать одновременно работающие blue и green ноды;
- не переключать load balancer на частично обновлённую группу;
- сохранять возможность быстрого rollback на предыдущую группу;
- не раскрывать credentials, внутренние адреса и тексты ошибок зависимостей.

Health endpoints не являются полным end-to-end тестом доставки событий. Путь
`publish -> JetStream -> consumer -> Redis dedup -> handler -> ACK` проверяется
отдельным deployment smoke test.

## Термины

| Термин | Значение |
|---|---|
| Liveness | Процесс и HTTP runtime способны ответить на запрос |
| Readiness | Нода может безопасно принимать новые HTTP и WebSocket-соединения |
| Release version | Человекочитаемая SemVer из `Cargo.toml` |
| Release revision | Полный Git commit SHA, вшитый в binary при сборке |
| Node ID | Стабильный `REALTIME_NODE_ID` конкретной realtime-ноды |
| Deployment slot | Логическая группа `single`, `blue` или `green` |
| Candidate slot | Неактивный slot, в который устанавливается новый release |
| Active slot | Slot, на который load balancer направляет клиентский трафик |

## HTTP-контракт

Приложение предоставляет два независимых endpoint:

| Endpoint | Успешный статус | Назначение |
|---|---:|---|
| `GET /health/live` | `200 OK` | Проверить только процесс и HTTP runtime |
| `GET /health/ready` | `200 OK` | Проверить возможность принимать новый трафик |

`GET /health/ready` возвращает `503 Service Unavailable`, если хотя бы одно
обязательное условие readiness не выполнено.

Оба endpoint возвращают:

```http
Content-Type: application/json
Cache-Control: no-store
```

Другие HTTP-методы получают стандартный `405 Method Not Allowed`.

### Liveness

Пример ответа `GET /health/live`:

```json
{
  "data": {
    "schemaVersion": 1,
    "status": "alive",
    "node": {
      "id": "realtime-4c5d5d83-37dc-4b7d-93b0-75229bf5ff50",
      "slot": "green"
    },
    "release": {
      "version": "0.0.3-alpha",
      "revision": "4f37cce87ba55e3d5ca31a52ca9d8f2058363be4"
    }
  }
}
```

Liveness не обращается к Redis, NATS, JetStream или OTel Collector. Недоступная
внешняя зависимость не должна создавать restart storm приложений, которые сами
по себе исправны.

### Readiness

Пример успешного ответа `GET /health/ready`:

```json
{
  "data": {
    "schemaVersion": 1,
    "status": "ready",
    "node": {
      "id": "realtime-4c5d5d83-37dc-4b7d-93b0-75229bf5ff50",
      "slot": "green"
    },
    "release": {
      "version": "0.0.3-alpha",
      "revision": "4f37cce87ba55e3d5ca31a52ca9d8f2058363be4"
    },
    "checks": {
      "traffic": "accepting",
      "redis": "up",
      "jetstream": "up",
      "consumer": "up"
    }
  }
}
```

Пример ответа неготовой ноды:

```http
HTTP/1.1 503 Service Unavailable
Content-Type: application/json
Cache-Control: no-store
```

```json
{
  "data": {
    "schemaVersion": 1,
    "status": "not_ready",
    "node": {
      "id": "realtime-4c5d5d83-37dc-4b7d-93b0-75229bf5ff50",
      "slot": "green"
    },
    "release": {
      "version": "0.0.3-alpha",
      "revision": "4f37cce87ba55e3d5ca31a52ca9d8f2058363be4"
    },
    "checks": {
      "traffic": "accepting",
      "redis": "up",
      "jetstream": "down",
      "consumer": "up"
    }
  }
}
```

Build и node metadata возвращаются даже при `503`. Это позволяет deployment
controller отличить новую ноду, которая ещё запускается, от старого контейнера.

### Допустимые значения checks

| Check | Значения |
|---|---|
| `traffic` | `accepting`, `draining` |
| `redis` | `up`, `down` |
| `jetstream` | `up`, `down`, `disabled` |
| `consumer` | `up`, `down`, `disabled` |

В режиме `EVENT_BUS_DRIVER=local` значения `jetstream` и `consumer` равны
`disabled` и не делают общий результат неуспешным. Redis остаётся обязательным,
поскольку используется приложением независимо от распределённого EventBus.

## Идентичность release и ноды

### `schemaVersion`

`schemaVersion` версионирует JSON-контракт health, а не приложение. Deployment
controller обязан отклонить неизвестную версию схемы вместо попытки угадать
значение новых полей.

Первая версия контракта имеет значение `1`.

### `release.version`

Источник — compile-time значение Cargo:

```rust
const VERSION: &str = env!("CARGO_PKG_VERSION");
```

SemVer удобна человеку, но не используется как deployment semaphore. Несколько
коммитов могут иметь одинаковую версию из `Cargo.toml`.

### `release.revision`

Источник — полный `GITHUB_SHA`, переданный в Docker build и вшитый в binary:

```rust
const REVISION: &str = match option_env!("APP_BUILD_REVISION") {
  Some(revision) => revision,
  None => "development",
};
```

Для Docker builder stage:

```dockerfile
ARG APP_BUILD_REVISION=development
ENV APP_BUILD_REVISION=$APP_BUILD_REVISION
```

Для `docker/build-push-action`:

```yaml
build-args: |
  APP_BUILD_REVISION=${{ github.sha }}
```

Значение не передаётся через production `.env`. Иначе старый binary можно
запустить с новой переменной окружения, и health ошибочно сообщит revision
нового релиза.

Значение `development` допустимо только для локальной сборки. Production-процесс
с такой revision завершает запуск с configuration error.

Используется полный SHA, а не сокращённый: сокращение удобно только для UI и
логов, но не для машинного сравнения.

### `node.id`

Источник — `REALTIME_NODE_ID`.

В production идентификатор создаётся `deploy/install-node-identity.sh` и
хранится в отдельном `node.env`. Общий `.env` заменяется при deployment и не
должен быть источником одинакового `REALTIME_NODE_ID` для нескольких серверов
или слотов.

Для локального single-node окружения `REALTIME_NODE_ID` может находиться в
локальном `.env`.

Целевой health-контракт требует непустой и валидный `REALTIME_NODE_ID` во всех
режимах EventBus. Отсутствующее или невалидное значение является ошибкой
запуска. В локальном режиме допустимо явное значение `local`, если одновременно
работает только один процесс.

### `node.slot`

Источник — runtime-переменная:

```env
DEPLOYMENT_SLOT=single
```

Если переменная отсутствует, используется `single`. Любое значение кроме
`single`, `blue` или `green` является ошибкой запуска. В настоящем blue-green
deployment slot указывается явно и не использует default.

Разрешённые значения:

- `single` — текущий single-host deployment;
- `blue` — blue slot;
- `green` — green slot.

Slot описывает принадлежность ноды к deployment-группе. Он не означает, что
группа сейчас active: источник истины о маршрутизации находится в load balancer
и deployment state.

## Семантика readiness

Readiness успешна только при одновременном выполнении всех обязательных условий:

```text
traffic == accepting
AND redis == up
AND (
  EVENT_BUS_DRIVER == local
  OR (jetstream == up AND consumer == up)
)
```

### Traffic state

Состояние при обычной работе — `accepting`.

При начале graceful shutdown приложение сначала переводит traffic state в
`draining`. Readiness сразу становится `503`, но liveness продолжает возвращать
`200`, пока HTTP runtime работает. После исключения ноды из load balancer
существующие WebSocket-соединения закрываются в пределах drain timeout.

Traffic state относится ко всему приложению и не является состоянием EventBus.

### Redis

Проверка выполняет `PING` через существующий `RedisClient` и принимает только
успешный ответ `PONG`.

Ошибка соединения, ошибка команды или timeout дают `redis=down` и общий `503`.
Текст ошибки и Redis URL не возвращаются клиенту.

### JetStream

Проверки Core NATS `connection_state()` или `flush()` недостаточны. Core NATS
может отвечать, когда metadata, stream или consumer JetStream недоступны.

Readiness выполняет ограниченный по времени read-only probe:

1. Запрашивает свежий `STREAM.INFO` настроенного stream.
2. Запрашивает свежий `CONSUMER.INFO` durable consumer текущей ноды.
3. Проверяет существование ресурсов и совместимость важных полей конфигурации.

Probe не вызывает `ensure_stream()`, не создаёт consumer, не ремонтирует
инфраструктуру и не публикует synthetic event.

### Incoming consumer

EventBus runtime публикует наблюдаемое состояние через cloneable readiness
handle:

| Состояние | Значение check | Readiness |
|---|---|---|
| `Disabled` | `disabled` | Не влияет в local mode |
| `Starting` | `down` | `503` |
| `Running` | `up` | Зависит от остальных checks |
| `Failed` | `down` | `503`, затем процесс завершается |

Для передачи последнего состояния подходит `tokio::sync::watch`. Состояние
`Starting` сохраняется, пока worker фактически не начал polling delivery stream.
Worker публикует startup barrier после первого poll, который вернул валидную
delivery либо `Pending` и зарегистрировал ожидание следующей delivery. Если
первый poll сразу возвращает ошибку или закрытый stream, состояние переходит
напрямую в `Failed`, без короткого окна ложной readiness.

Terminal transition `Failed` публикуется до возврата ошибки supervisor-у.

Во время автоматического reconnect runtime может оставаться в `Running`, но
JetStream probe возвращает `down`. После восстановления JetStream readiness
автоматически возвращается в `200`.

### Timeout и нагрузка

Redis и EventBus probes запускаются параллельно. Общий timeout readiness должен
быть порядка одной-двух секунд и быть меньше timeout Docker/LB healthcheck.

Health endpoint не должен:

- ждать бесконечного reconnect;
- создавать новые background tasks на каждый timeout;
- записывать probe keys в Redis;
- публиковать события в JetStream;
- писать error log на каждый запрос балансировщика.

Подробная ошибка логируется структурированно при смене состояния или через
ограниченное по частоте сообщение.

OTel Collector не входит в readiness. Потеря телеметрии не мешает приложению
обрабатывать клиентский трафик.

## Граница компонентов

Целевая структура:

```text
src/app/health.rs
  ApplicationHealth
  ReadinessReport
  NodeInfo
  ReleaseInfo

src/app/providers/event_bus/health.rs
  EventBusReadinessHandle
  EventBusRuntimeState

crates/nats-client/src/health.rs
  NatsClient::verify_topology(...)

src/app/http/routes/health.rs
  live()
  ready()
```

`ApplicationHealth` хранит:

- `Arc<RedisClient>`;
- `EventBusReadinessHandle`;
- node и release metadata;
- application-level traffic state;
- timeout readiness probes.

`EventBusProvider` создаёт `EventBusReadinessHandle` вместе с runtime. Handle
инкапсулирует режим EventBus, optional `Arc<NatsClient>`, ожидаемые
`StreamConfig`/`ConsumerConfig` и `watch`-состояние worker. Поэтому
`ApplicationHealth` может выполнить единый `event_bus.check()` и при этом не
получает `NatsClient` напрямую.

`AppState` получает `Arc<ApplicationHealth>` как обычный peer-сервис. Сам
`NatsClient` не добавляется в `AppState`: он остаётся внутренней зависимостью
EventBus.

HTTP handler только преобразует `ReadinessReport` в JSON и status code. Он не
содержит логику reconnect, создания stream или управления consumer.

## Docker Compose

Container healthcheck приложения проверяет readiness, потому что
`docker compose up --wait` должен ждать не только запущенный процесс, но и
готовность зависимостей и consumer loop:

```yaml
services:
  app:
    healthcheck:
      test:
        - CMD
        - curl
        - --fail
        - --silent
        - --show-error
        - --connect-timeout
        - "1"
        - --max-time
        - "2"
        - http://127.0.0.1:4008/health/ready
      interval: 5s
      timeout: 3s
      retries: 6
      start_period: 15s
```

Runtime image должен содержать `curl` либо отдельный healthcheck subcommand
приложения. Shell-трюки с `/dev/tcp` не используются.

Docker Compose не перезапускает контейнер только из-за статуса `unhealthy`.
Healthcheck используется для первоначального `--wait`, диагностики и внешнего
deployment gate. `restart: unless-stopped` срабатывает при завершении процесса.

## Load balancer

Load balancer проверяет только:

```text
GET /health/ready
expected status: 200
```

Рекомендуемые исходные параметры:

- interval: 5 секунд;
- timeout: 2 секунды;
- unhealthy threshold: 3;
- healthy threshold: 2.

Load balancer обычно не анализирует JSON release metadata. Сравнение revision,
node ID и slot выполняет deployment controller.

Liveness используется мониторингом процесса, но не является условием допуска
ноды к клиентскому трафику.

## Blue-green deployment semaphore

### Основной принцип

Readiness отвечает на вопрос «может ли нода принимать трафик сейчас».
Deployment semaphore отвечает на другой вопрос: «принадлежит ли готовая нода
нужному release и candidate slot».

Поэтому здоровая нода со старой revision продолжает возвращать `200 ready`.
Deployment controller отклоняет её из-за несовпадения `release.revision`, но не
меняет её operational health.

### Условие допуска candidate slot

Пусть `TARGET_REVISION` — полный SHA коммита, который запустил workflow.
Candidate slot можно переключить в active только если:

```text
candidate_is_releasable =
  number_of_nodes == expected_number_of_nodes
  AND set(node.id) == expected_node_ids
  AND every node.id is unique
  AND every node.slot == candidate_slot
  AND every schemaVersion == supportedSchemaVersion
  AND every release.revision == TARGET_REVISION
  AND every GET /health/ready returns 200
```

Версия из `Cargo.toml` не участвует в машинном сравнении.

### Порядок deployment

1. GitHub Actions `concurrency` не допускает одновременно два production
   deployment workflow.
2. Deployment controller определяет active slot из состояния load balancer.
3. Новый image устанавливается в противоположный candidate slot.
4. Каждая candidate-нода опрашивается напрямую, минуя общий VIP.
5. Controller проверяет JSON schema, node ID, slot, revision и readiness.
6. Для каждой ноды требуются три последовательных успешных полных раунда.
7. Через candidate upstream выполняется отдельный smoke test.
8. Load balancer атомарно переключает маршрутизацию новых соединений на
   candidate slot.
9. Новый active slot наблюдается в течение установленного soak period.
10. Предыдущий slot остаётся доступен для rollback.
11. После rollback window предыдущий slot переводится в draining и
    останавливается.

Опрашивать общий публичный IP несколько раз недостаточно: load balancer может
возвращать одну и ту же ноду из-за connection reuse, кеша или выбранного
алгоритма балансировки. Controller использует private address каждой ноды либо
выполняет запрос через SSH на её loopback endpoint.

Текущий workflow использует один `DEPLOY_HOST`. Для HA и blue-green он должен
получить явный inventory candidate-нод: address, ожидаемый node ID, slot и
backend port каждой ноды. Неизвестная или отсутствующая нода закрывает gate.

Параметры deployment gate фиксированы:

- все ожидаемые ноды опрашиваются параллельно в одном раунде;
- timeout одного HTTP-запроса — 3 секунды;
- interval между раундами — 5 секунд;
- требуется 3 последовательных успешных раунда;
- общий deadline ожидания — 180 секунд;
- любой timeout, `503`, malformed JSON, неизвестная schema, unexpected или
  duplicate node ID, неверный slot или revision сбрасывает счётчик успешных
  раундов;
- по истечении deadline controller завершает deployment без переключения.

Один успешный раунд означает, что полный набор ответивших `node.id` точно равен
ожидаемому inventory и все условия `candidate_is_releasable` выполнены в рамках
этого раунда.

### Состояния slot

| Slot | Revision | Readiness | LB state | Действие |
|---|---|---|---|---|
| Blue | старая | ready | active | Продолжает обслуживать трафик |
| Green | новая | starting | inactive | Ждать readiness |
| Green | новая | ready | inactive | Выполнить gate и smoke test |
| Green | новая | ready | active | Наблюдать soak period |
| Blue | старая | draining | inactive | Сохранить для rollback или остановить |

Поле `node.slot` не является источником истины о колонке `LB state`.

### Mixed-version период

Атомарное изменение конфигурации load balancer относится только к новым
соединениям. Уже установленные WebSocket-соединения продолжают обслуживаться
предыдущим slot до завершения или drain timeout. В этот период одновременно
работают release N и N-1.

Каждый release обязан сохранять N/N-1 совместимость для:

- HTTP и WebSocket протокола;
- frontend и backend: открытая вкладка может использовать предыдущий bundle;
- JetStream event envelope и payload schemas;
- Redis session, presence и dedup keys/values;
- общих конфигурационных значений и namespace.

Изменения выполняются по expand/contract схеме: сначала добавляется совместимый
reader/writer, затем выкатываются все ноды, и только после закрытия rollback
window и завершения старых соединений удаляется поддержка прежнего формата.
Destructive migrations, удаление старой event schema или несовместимое изменение
Redis-формата до этого момента запрещены.

Успешный deployment semaphore подтверждает identity и operational readiness
candidate slot, но сам по себе не доказывает совместимость N/N-1.

### Rollback

При ошибке до переключения candidate slot не добавляется в load balancer.

При ошибке после переключения:

1. Проверяется readiness предыдущего slot.
2. Load balancer атомарно возвращает новые соединения на предыдущий slot.
3. Неуспешный slot переводится в draining.
4. Сохраняются health responses, container logs и результаты smoke test.

Rollback использует предыдущий проверенный image SHA или image digest, а не
mutable tag `production`.

## `REALTIME_NODE_ID` при blue-green

Два одновременно работающих процесса приложения не могут использовать один
`REALTIME_NODE_ID`. Идентификатор входит в имя durable consumer и область
consumer-side дедупликации. Общий ID заставит процессы разделить один consumer,
поэтому событие класса `AllNodes` может попасть только одному из них.

Правила:

- у blue и green процессов разные node ID;
- на одном сервере каждый slot имеет отдельный deployment directory и
  `node.env`;
- каждый одновременно работающий slot имеет отдельный Compose project,
  backend port и upstream load balancer;
- node ID сохраняется при обычном рестарте контейнера внутри того же slot;
- новый node ID не создаётся на каждый restart или каждый pull image;
- при окончательном выводе ноды её durable consumer удаляется;
- dormant consumer не оставляется бессрочно накапливать backlog.

Если предыдущий slot сохраняется для rollback, его consumer остаётся активным в
течение rollback window. После окончательного вывода slot consumer удаляется.
При следующем вводе slot создаётся новый consumer с актуальной delivery policy,
без воспроизведения backlog периода простоя.

## Image digest

Полный Git SHA определяет исходный код, но registry tag технически может быть
перезаписан. Следующее усиление deployment:

1. Получать digest из результата `docker/build-push-action`.
2. Устанавливать `APP_IMAGE=repository@sha256:...`.
3. Сверять фактический container image через Docker API или `docker inspect`.

Приложению не передаётся Docker socket. Image digest не считается
авторитетным полем app health: binary не может самостоятельно доказать digest
контейнера, внутри которого он запущен.

`release.revision` остаётся полезным для наблюдаемости и проверки соответствия
binary ожидаемому коммиту.

## Безопасность ответа

Health response может содержать:

- SemVer;
- полный Git SHA;
- node ID;
- deployment slot;
- стабильные статусы `up`, `down`, `disabled`, `accepting`, `draining`.

Health response не содержит:

- Redis или NATS URL;
- usernames, passwords, tokens и API keys;
- stream, durable consumer или subject names;
- raw error messages;
- filesystem paths;
- build-host information.

Подробные причины отказа доступны в structured logs и metrics. Если disclosure
Git SHA или node ID нежелателен для публичного клиента, Nginx ограничивает
health endpoints адресами load balancer и административной сети.

Для HA предпочтителен именно private/direct-node доступ. Текущий catch-all
proxy Nginx опубликует новые маршруты наружу автоматически, поэтому до включения
контракта необходимо добавить отдельную access policy для `/health/live` и
`/health/ready` либо принять публичное раскрытие перечисленных metadata.

## Тестирование

### HTTP contract tests

- liveness возвращает точный `200`, JSON и `Cache-Control: no-store`;
- liveness не вызывает probes зависимостей;
- readiness со всеми успешными checks возвращает `200`;
- каждый обязательный `down` отдельно приводит к `503`;
- local EventBus с `disabled` checks остаётся ready;
- `draining` всегда приводит к `503`;
- timeout probe приводит к `503` за ограниченное время;
- body `503` всё равно содержит node и release metadata;
- raw error, URL или secret из fake probe отсутствуют в response;
- неподдерживаемый HTTP-метод получает `405`.

### Runtime tests

- JetStream runtime проходит `Starting -> Running`;
- readiness не становится `200` до startup barrier первого poll consumer;
- terminal consumer error переводит runtime в `Failed`;
- local runtime сообщает `Disabled`;
- отсутствие `REALTIME_NODE_ID` завершает запуск с configuration error;
- отсутствие `DEPLOYMENT_SLOT` даёт `single`, неизвестное значение завершает
  запуск с configuration error;
- production binary с revision `development` не запускается;
- временная недоступность JetStream даёт `503` при живом HTTP runtime;
- восстановление зависимости возвращает readiness в `200` либо приводит к
  успешному restart контейнера, если runtime завершился.

### Compose tests

- `docker compose up --wait` ждёт healthcheck приложения;
- healthy Compose stack возвращает `live=200` и `ready=200`;
- остановка Redis не меняет liveness, но делает приложение unhealthy;
- потеря NATS/JetStream не оставляет ноду в состоянии ready;
- после восстановления или restart нода возвращается в ready;
- выполняется отдельный полный message smoke test.

### Deployment gate tests

- все candidate-ноды с target revision разрешают переключение;
- одна нода со старой revision запрещает переключение;
- duplicate node ID запрещает переключение;
- неожиданный slot запрещает переключение;
- неизвестный `schemaVersion` запрещает переключение;
- `503` хотя бы одной ноды запрещает переключение;
- запросы к общему VIP не используются как доказательство обновления всех нод;
- gate требует три полных успешных раунда и сбрасывает серию при любом отказе;
- gate завершается без switch после общего deadline;
- ошибка после switch возвращает трафик на предыдущий slot.

### N/N-1 compatibility tests

- предыдущий frontend bundle работает с новым backend;
- новый frontend bundle сохраняет необходимую совместимость с предыдущим
  backend во время rollback window;
- release N и N-1 взаимно понимают события, публикуемые во время mixed-version
  периода;
- release N читает существующие Redis session, presence и dedup values;
- rollback не требует destructive migration или восстановления удалённой event
  schema.

## Критерии готовности

Доработка считается завершённой, когда:

- оба endpoint реализованы и зарегистрированы;
- revision вшивается в binary на этапе Docker build;
- production health никогда не сообщает `development` revision;
- node ID берётся из отдельной identity конкретной ноды/slot;
- readiness проверяет Redis, JetStream topology и consumer runtime;
- app healthcheck подключён в Compose;
- `docker compose up --wait` ожидает readiness приложения;
- load balancer использует `/health/ready`;
- deployment controller сравнивает полный SHA каждой candidate-ноды;
- blue и green ноды имеют разные `REALTIME_NODE_ID`;
- реализован lifecycle удаления retired durable consumer;
- unit, integration и deployment gate tests проходят;
- полный live smoke test выполнен хотя бы в staging.

## Текущее состояние реализации

| Возможность | Состояние |
|---|---|
| Пустой `health()` handler | Есть |
| Зарегистрированные health routes | Нет |
| `RedisClient::ping()` | Есть |
| Наблюдаемое состояние EventBus runtime | Нет |
| Read-only JetStream topology probe | Нет |
| App healthcheck в Compose | Нет |
| Cargo SemVer | Есть |
| Docker image tag по полному commit SHA | Есть |
| OCI label с Git revision | Есть |
| Git revision внутри binary | Нет |
| Стабильный `node.env` single-host ноды | Есть |
| Deployment inventory из нескольких нод | Нет |
| Отдельные blue/green slots | Нет |
| Graceful WebSocket drain | Нет |
| Blue-green deployment controller | Нет |

## Рекомендуемый порядок реализации

1. Добавить release и node metadata.
2. Реализовать liveness без зависимостей.
3. Добавить `EventBusReadinessHandle` и runtime state transitions.
4. Добавить read-only JetStream topology probe.
5. Реализовать `ApplicationHealth` и readiness.
6. Добавить HTTP contract tests.
7. Добавить app healthcheck в Docker Compose.
8. Добавить failure/recovery smoke tests.
9. Реализовать application-level draining.
10. Реализовать два deployment slot и прямой per-node gate.
11. Добавить atomic LB switch и rollback.
12. Перейти от image tag к image digest.

## Связанные документы

- [Архитектура серверов и deployment](./deployment.md)
- [Автономный и кластерный режимы](./clustering.md)
- [Текущий Compose](../compose.yml)
- [GitHub Actions deployment](../.github/workflows/deploy.yml)
