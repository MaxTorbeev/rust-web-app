<script setup lang="ts">
import { UsersIcon } from '@lucide/vue'
import { Badge } from '@/components/ui/badge'
import type { ChatMember } from './types'

defineProps<{
  members: ChatMember[]
  clientId: string
}>()
</script>

<template>
  <div class="flex items-center justify-between gap-3 px-2 pb-4">
    <span class="flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
      <UsersIcon class="size-4" aria-hidden="true" />
      Участники
    </span>
    <Badge variant="secondary">{{ members.length }}</Badge>
  </div>

  <ul class="min-h-0 flex-1 space-y-1 overflow-y-auto" aria-label="Участники в сети">
    <li
      v-for="member in members"
      :key="member.connectionId"
      class="flex min-w-0 items-center gap-3 rounded-lg px-2 py-2"
    >
      <span
        class="relative grid size-9 shrink-0 place-items-center rounded-lg bg-accent font-semibold text-accent-foreground"
        aria-hidden="true"
      >
        {{ member.name.slice(0, 1).toUpperCase() }}
        <span class="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full bg-online ring-2 ring-muted" />
      </span>
      <span class="min-w-0">
        <strong class="block truncate text-sm font-medium">{{ member.name }}</strong>
        <small class="block truncate text-xs text-muted-foreground">
          {{ member.clientId === clientId ? 'Это вы' : member.clientId }}
        </small>
      </span>
    </li>
  </ul>
</template>
