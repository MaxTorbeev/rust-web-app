<script setup lang="ts">
import { LogOutIcon, MessageCircleIcon } from '@lucide/vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { ChatStatus } from './types'

defineProps<{
  status: ChatStatus
  statusLabel: string
  connected: boolean
}>()

const emit = defineEmits<{
  disconnect: []
}>()
</script>

<template>
  <div class="flex min-w-0 items-center gap-3">
    <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
      <MessageCircleIcon class="size-5" aria-hidden="true" />
    </span>
    <h1 class="truncate text-lg font-semibold tracking-tight">Realtime Chat</h1>
  </div>

  <div class="flex items-center gap-2">
    <Badge
      variant="outline"
      class="h-8 gap-2 bg-card px-3 text-muted-foreground"
      :data-state="status"
      role="status"
      aria-live="polite"
    >
      <span class="status-dot size-2 rounded-full bg-subtle" aria-hidden="true" />
      {{ statusLabel }}
    </Badge>
    <Button
      v-if="connected"
      variant="outline"
      size="sm"
      aria-label="Отключиться"
      @click="emit('disconnect')"
    >
      <LogOutIcon aria-hidden="true" />
      <span class="hidden sm:inline">Отключиться</span>
    </Button>
  </div>
</template>
