<script setup lang="ts">
import { LoaderCircleIcon, SendHorizontalIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'

const props = defineProps<{
  canSend: boolean
  sending: boolean
  typingLabel: string
}>()

const emit = defineEmits<{
  submit: []
  draftInput: []
}>()

const draft = defineModel<string>({ required: true })

function submit(): void {
  if (props.canSend) emit('submit')
}
</script>

<template>
  <div
    class="h-6 w-full min-w-0 truncate px-1 pt-1 text-xs text-muted-foreground"
    role="status"
    aria-live="polite"
    :title="typingLabel"
  >
    {{ typingLabel }}
  </div>
  <form class="flex min-w-0 items-end gap-2" @submit.prevent="submit">
    <Textarea
      v-model="draft"
      class="max-h-32 min-h-11 resize-none rounded-xl py-3"
      rows="1"
      maxlength="1500"
      placeholder="Напишите сообщение"
      aria-label="Сообщение"
      @input="emit('draftInput')"
      @keydown.enter.exact.prevent="submit"
    />
    <Button
      class="size-11 shrink-0 rounded-xl"
      size="icon-lg"
      type="submit"
      :disabled="!canSend"
      aria-label="Отправить сообщение"
    >
      <LoaderCircleIcon v-if="sending" class="animate-spin" aria-hidden="true" />
      <SendHorizontalIcon v-else aria-hidden="true" />
    </Button>
  </form>
</template>
