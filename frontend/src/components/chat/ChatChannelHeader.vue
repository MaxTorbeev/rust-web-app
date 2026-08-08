<script setup lang="ts">
import { Badge } from '@/components/ui/badge'

defineProps<{
  channelName: string
  eventName: string
  clientId: string
}>()
</script>

<template>
  <div class="min-w-0">
    <h2 class="truncate font-semibold"># {{ channelName }}</h2>
    <p class="truncate text-xs text-muted-foreground">{{ eventName }}</p>
  </div>
  <Badge variant="secondary" class="hidden max-w-56 truncate sm:inline-flex">
    {{ clientId }}
  </Badge>
</template>
