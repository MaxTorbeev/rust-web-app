<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, type Ref } from 'vue'
import type {
  ConnectionState,
  ConnectionStateChange,
  InboundMessage,
  PresenceMessage,
} from 'ably'
import {
  RealtimeChat,
  chatPayload,
  presencePayload,
  type ChatPayload,
  type PresencePayload,
} from './realtime'

const CLIENT_ID_KEY = 'realtime-chat-client-id'
const MAX_MESSAGES = 250
const TYPING_TIMEOUT = 1400
const query = new URLSearchParams(location.search)

const statusLabels: Record<ConnectionState, string> = {
  initialized: 'Не подключён',
  connecting: 'Подключение',
  connected: 'В сети',
  disconnected: 'Соединение потеряно',
  suspended: 'Соединение приостановлено',
  closing: 'Отключение',
  closed: 'Отключён',
  failed: 'Ошибка',
}

const timeFormatter = new Intl.DateTimeFormat('ru', {
  hour: '2-digit',
  minute: '2-digit',
})

interface Member extends PresencePayload {
  clientId: string
  connectionId: string
}

const form = reactive({
  applicationId: 'staging',
  channelName: query.get('channel') ?? 'private:chat.example',
  eventName: query.get('event') ?? 'chat.message',
  clientId: localStorage.getItem(CLIENT_ID_KEY) ?? `guest-${crypto.randomUUID().slice(0, 8)}`,
  displayName: '',
})

const status = ref<ConnectionState>('initialized')
const error = ref('')
const draft = ref('')
const messages = ref<ChatPayload[]>([])
const members = reactive(new Map<string, Member>())
const connecting = ref(false)
const sending = ref(false)
const messageList = ref<HTMLElement>()

let chat: RealtimeChat | null = null
let typing = false
let typingTimer: number | undefined

const connected = computed(() => status.value === 'connected')
const canConnect = computed(() => Object.values(form).every(Boolean))
const canSend = computed(() => draft.value.trim() !== '' && !sending.value)
const statusLabel = computed(() => statusLabels[status.value])
const memberList = computed(() => [...members.values()]
  .sort((left, right) => left.name.localeCompare(right.name, 'ru')))
const typingLabel = computed(() => {
  const names = memberList.value
    .filter((member) => member.typing && member.clientId !== form.clientId)
    .map((member) => member.name)

  if (names.length === 0) return ''
  if (names.length === 1) return `${names[0]} печатает…`

  return `Печатают: ${names.join(', ')}`
})

async function connect(): Promise<void> {
  chat?.client.close()
  chat = null
  members.clear()
  messages.value = []

  await withPending(connecting, async () => {
    chat = new RealtimeChat({
      ...form,
      onConnectionChange,
      onMessage: receiveMessage,
      onPresence: receivePresence,
    })

    await chat.connect()
    localStorage.setItem(CLIENT_ID_KEY, form.clientId)
    updateLocation()
  })
}

async function disconnect(): Promise<void> {
  resetTyping()
  members.clear()
  status.value = 'closed'

  const currentChat = chat
  chat = null

  if (currentChat === null) return

  try {
    await currentChat.close()
  } catch (reason) {
    currentChat.client.close()
    showError(reason)
  }
}

async function sendMessage(): Promise<void> {
  const currentChat = chat
  if (currentChat === null) return

  await withPending(sending, async () => {
    await currentChat.publish({
      id: crypto.randomUUID(),
      text: draft.value.trim(),
      senderId: form.clientId,
      senderName: form.displayName,
      sentAt: Date.now(),
    })

    draft.value = ''
    clearTypingTimer()
    setTyping(false)
  })
}

function onConnectionChange(change: ConnectionStateChange): void {
  status.value = change.current

  if (change.current !== 'connected') members.clear()
  if (change.reason) showError(change.reason)
}

function receiveMessage(message: InboundMessage): void {
  messages.value.push(chatPayload(message))

  if (messages.value.length > MAX_MESSAGES) messages.value.shift()

  void nextTick(() => messageList.value?.scrollTo({
    top: messageList.value.scrollHeight,
    behavior: 'smooth',
  }))
}

function receivePresence(message: PresenceMessage): void {
  const connectionId = message.connectionId ?? message.clientId

  if (message.action === 'leave' || message.action === 'absent') {
    members.delete(connectionId)
    return
  }

  members.set(connectionId, {
    clientId: message.clientId,
    connectionId,
    ...presencePayload(message),
  })
}

function onDraftInput(): void {
  clearTypingTimer()

  const hasText = draft.value.trim() !== ''
  setTyping(hasText)

  if (hasText) {
    typingTimer = window.setTimeout(() => setTyping(false), TYPING_TIMEOUT)
  }
}

function setTyping(value: boolean): void {
  if (typing === value || chat === null) return

  typing = value
  void chat.setTyping(value).catch(showError)
}

function clearTypingTimer(): void {
  if (typingTimer !== undefined) window.clearTimeout(typingTimer)
  typingTimer = undefined
}

function resetTyping(): void {
  clearTypingTimer()
  typing = false
}

async function withPending(pending: Ref<boolean>, task: () => Promise<void>): Promise<void> {
  pending.value = true
  error.value = ''

  try {
    await task()
  } catch (reason) {
    showError(reason)
  } finally {
    pending.value = false
  }
}

function showError(reason: unknown): void {
  error.value = reason instanceof Error ? reason.message : String(reason)
}

function updateLocation(): void {
  const params = new URLSearchParams(location.search)

  params.set('channel', form.channelName)
  params.set('event', form.eventName)
  history.replaceState(null, '', `${location.pathname}?${params}`)
}

const formatTime = (timestamp: number): string => timeFormatter.format(timestamp)

onBeforeUnmount(() => {
  resetTyping()
  chat?.client.close()
})
</script>

<template>
  <main class="page-shell">
    <section class="chat-card">
      <header class="topbar">
        <div>
          <p class="eyebrow">Native AblyJS</p>
          <h1>Realtime chat</h1>
        </div>

        <div class="connection-state" :data-state="status">
          <span class="status-dot" />
          {{ statusLabel }}
        </div>
      </header>

      <form v-if="!connected" class="connection-form" @submit.prevent="connect">
        <div class="form-intro">
          <h2>Подключение</h2>
        </div>

        <label>
          <span>Application ID</span>
          <select v-model="form.applicationId" required>
            <option value="staging">staging</option>
          </select>
        </label>

        <label>
          <span>Канал</span>
          <input
            v-model.trim="form.channelName"
            autocomplete="off"
            placeholder="private:chat.example"
            required
          >
        </label>

        <label>
          <span>Событие</span>
          <input
            v-model.trim="form.eventName"
            autocomplete="off"
            required
          >
        </label>

        <div class="form-row">
          <label>
            <span>Client ID</span>
            <input v-model.trim="form.clientId" autocomplete="off" required>
          </label>

          <label>
            <span>Имя</span>
            <input
              v-model.trim="form.displayName"
              autocomplete="name"
              maxlength="48"
              required
            >
          </label>
        </div>

        <button class="primary-button" type="submit" :disabled="!canConnect || connecting">
          {{ connecting ? 'Подключаем…' : 'Войти в чат' }}
        </button>
      </form>

      <div v-else class="workspace">
        <aside class="members-panel">
          <div class="panel-heading">
            <span>Активность</span>
            <strong>{{ memberList.length }}</strong>
          </div>

          <ul class="member-list">
            <li v-for="member in memberList" :key="member.connectionId">
              <span class="avatar">{{ member.name.slice(0, 1).toUpperCase() }}</span>
              <span class="member-name">
                {{ member.name }}
                <small v-if="member.clientId === form.clientId">это вы</small>
              </span>
              <span class="online-dot" />
            </li>
          </ul>

          <button class="text-button" type="button" @click="disconnect">
            Отключиться
          </button>
        </aside>

        <section class="messages-panel">
          <div class="channel-heading">
            <div>
              <span class="channel-prefix">#</span>
              <strong>{{ form.channelName }}</strong>
            </div>
            <span>{{ form.clientId }}</span>
          </div>

          <div ref="messageList" class="message-list" aria-live="polite">
            <div v-if="messages.length === 0" class="empty-state">
              <span>✦</span>
              <h2>Канал подключён</h2>
              <p>Напишите первое сообщение. История начинается с текущей сессии.</p>
            </div>

            <article
              v-for="message in messages"
              :key="message.id"
              class="message"
              :class="{ 'message-own': message.senderId === form.clientId }"
            >
              <div class="message-meta">
                <strong>{{ message.senderName }}</strong>
                <time :datetime="new Date(message.sentAt).toISOString()">
                  {{ formatTime(message.sentAt) }}
                </time>
              </div>
              <p>{{ message.text }}</p>
            </article>
          </div>

          <div class="typing-line">{{ typingLabel }}</div>

          <form class="composer" @submit.prevent="sendMessage">
            <textarea
              v-model="draft"
              rows="1"
              maxlength="1500"
              placeholder="Сообщение"
              @input="onDraftInput"
              @keydown.enter.exact.prevent="sendMessage"
            />
            <button type="submit" :disabled="!canSend" aria-label="Отправить сообщение">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m4 4 17 8-17 8 3-8-3-8Zm3.8 7h8.3L6.7 6.6 7.8 11Zm0 2-1.1 4.4 9.4-4.4H7.8Z" />
              </svg>
            </button>
          </form>
        </section>
      </div>

      <p v-if="error" class="error-banner" role="alert">{{ error }}</p>
    </section>
  </main>
</template>
