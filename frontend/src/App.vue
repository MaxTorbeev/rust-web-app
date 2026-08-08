<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, type Ref } from 'vue'
import type {
  ConnectionState,
  ConnectionStateChange,
  InboundMessage,
  PresenceMessage,
} from 'ably'
import {
  ChatChannelHeader,
  ChatComposer,
  ChatConnectionForm,
  ChatHeader,
  ChatMembers,
  ChatMessages,
  type ChatConnectionFormModel,
  type ChatMember,
  type ChatMessage,
} from '@/components/chat'
import { ChatLayout, ChatRoomLayout } from '@/layouts'
import {
  RealtimeChat,
  chatPayload,
  presencePayload,
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

const form = ref<ChatConnectionFormModel>({
  applicationId: 'staging',
  channelName: query.get('channel') ?? 'private:chat.example',
  eventName: query.get('event') ?? 'chat.message',
  clientId: localStorage.getItem(CLIENT_ID_KEY) ?? `guest-${crypto.randomUUID().slice(0, 8)}`,
  displayName: '',
})

const status = ref<ConnectionState>('initialized')
const error = ref('')
const draft = ref('')
const messages = ref<ChatMessage[]>([])
const members = reactive(new Map<string, ChatMember>())
const connecting = ref(false)
const sending = ref(false)

let chat: RealtimeChat | null = null
let typing = false
let typingTimer: number | undefined

const connected = computed(() => status.value === 'connected')
const canConnect = computed(() => Object.values(form.value).every((value) => value.trim() !== ''))
const canSend = computed(() => draft.value.trim() !== '' && !sending.value)
const statusLabel = computed(() => statusLabels[status.value])
const memberList = computed(() => [...members.values()]
  .sort((left, right) => left.name.localeCompare(right.name, 'ru')))
const typingLabel = computed(() => {
  const names = memberList.value
    .filter((member) => member.typing && member.clientId !== form.value.clientId)
    .map((member) => member.name)

  if (names.length === 0) return ''
  if (names.length === 1) return `${names[0]} печатает…`

  const visibleNames = names.slice(0, 2)
  const hiddenCount = names.length - visibleNames.length

  return `Печатают: ${visibleNames.join(', ')}${hiddenCount > 0 ? ` и ещё ${hiddenCount}` : ''}`
})

async function connect(): Promise<void> {
  trimConnectionForm()
  if (!canConnect.value) return

  chat?.client.close()
  chat = null
  members.clear()
  messages.value = []

  await withPending(connecting, async () => {
    chat = new RealtimeChat({
      ...form.value,
      onConnectionChange,
      onMessage: receiveMessage,
      onPresence: receivePresence,
    })

    await chat.connect()
    localStorage.setItem(CLIENT_ID_KEY, form.value.clientId)
    updateLocation()
  })
}

function trimConnectionForm(): void {
  for (const field of Object.keys(form.value) as (keyof ChatConnectionFormModel)[]) {
    form.value[field] = form.value[field].trim()
  }
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
      senderId: form.value.clientId,
      senderName: form.value.displayName,
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

  params.set('channel', form.value.channelName)
  params.set('event', form.value.eventName)
  history.replaceState(null, '', `${location.pathname}?${params}`)
}

onBeforeUnmount(() => {
  resetTyping()
  chat?.client.close()
})
</script>

<template>
  <ChatLayout :show-header="connected">
    <template #header>
      <ChatHeader
        :status="status"
        :status-label="statusLabel"
        :connected="connected"
        @disconnect="disconnect"
      />
    </template>

    <ChatConnectionForm
      v-if="!connected"
      v-model="form"
      :can-connect="canConnect"
      :connecting="connecting"
      @submit="connect"
    />

    <ChatRoomLayout v-else>
      <template #members>
        <ChatMembers :members="memberList" :client-id="form.clientId" />
      </template>

      <template #header>
        <ChatChannelHeader
          :channel-name="form.channelName"
          :event-name="form.eventName"
          :client-id="form.clientId"
        />
      </template>

      <ChatMessages :messages="messages" :client-id="form.clientId" />

      <template #composer>
        <ChatComposer
          v-model="draft"
          :can-send="canSend"
          :sending="sending"
          :typing-label="typingLabel"
          @draft-input="onDraftInput"
          @submit="sendMessage"
        />
      </template>
    </ChatRoomLayout>

    <template #overlay>
      <p
        v-if="error"
        class="absolute inset-x-4 top-20 z-20 ml-auto max-w-lg rounded-lg border border-destructive/25 bg-red-50 px-4 py-3 text-sm text-red-800 shadow-sm"
        role="alert"
      >
        {{ error }}
      </p>
    </template>
  </ChatLayout>
</template>
