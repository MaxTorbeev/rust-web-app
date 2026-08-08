<script setup lang="ts">
import { MessageCircleIcon } from '@lucide/vue'
import { nextTick, ref, watch } from 'vue'
import { Bubble, BubbleContent } from '@/components/ui/bubble'
import type { ChatMessage } from './types'

const props = defineProps<{
  messages: ChatMessage[]
  clientId: string
}>()

const messageList = ref<HTMLElement>()
const timeFormatter = new Intl.DateTimeFormat('ru', {
  hour: '2-digit',
  minute: '2-digit',
})

const formatTime = (timestamp: number): string => timeFormatter.format(timestamp)
let stickToBottom = true

function rememberScrollPosition(): void {
  const list = messageList.value
  if (list === undefined) return

  stickToBottom = list.scrollHeight - list.scrollTop - list.clientHeight < 80
}

watch(() => props.messages.at(-1)?.id, async () => {
  const lastMessage = props.messages.at(-1)
  if (!stickToBottom && lastMessage?.senderId !== props.clientId) return

  await nextTick()
  messageList.value?.scrollTo({
    top: messageList.value.scrollHeight,
    behavior: 'smooth',
  })
})
</script>

<template>
  <div
    ref="messageList"
    class="flex h-full min-h-0 flex-col gap-5 overflow-y-auto px-4 py-5 sm:px-6"
    role="log"
    aria-label="Сообщения канала"
    aria-live="polite"
    aria-relevant="additions"
    @scroll.passive="rememberScrollPosition"
  >
    <div v-if="messages.length === 0" class="m-auto max-w-sm text-center">
      <span class="mx-auto grid size-12 place-items-center rounded-2xl bg-accent text-accent-foreground">
        <MessageCircleIcon class="size-6" aria-hidden="true" />
      </span>
      <h2 class="mt-4 text-lg font-semibold">Канал подключён</h2>
      <p class="mt-1.5 text-sm leading-relaxed text-muted-foreground">
        Напишите первое сообщение. История начинается с текущей сессии.
      </p>
    </div>

    <Bubble
      v-for="message in messages"
      :key="message.id"
      as="article"
      :align="message.senderId === clientId ? 'end' : 'start'"
      :variant="message.senderId === clientId ? 'default' : 'secondary'"
      class="max-w-[min(42rem,85%)]"
    >
      <div
        class="flex items-center gap-2 px-1 text-xs text-muted-foreground"
        :class="{ 'justify-end': message.senderId === clientId }"
      >
        <strong class="truncate font-medium">{{ message.senderName }}</strong>
        <time class="shrink-0" :datetime="new Date(message.sentAt).toISOString()">
          {{ formatTime(message.sentAt) }}
        </time>
      </div>
      <BubbleContent class="whitespace-pre-wrap">{{ message.text }}</BubbleContent>
    </Bubble>
  </div>
</template>
