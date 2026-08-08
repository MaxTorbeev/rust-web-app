<script setup lang="ts">
import { LoaderCircleIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { ChatConnectionFormModel } from './types'

defineProps<{
  canConnect: boolean
  connecting: boolean
}>()

const form = defineModel<ChatConnectionFormModel>({ required: true })

const emit = defineEmits<{
  submit: []
}>()
</script>

<template>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <form class="mx-auto grid w-full max-w-2xl gap-6 px-5 py-8 sm:px-8 sm:py-12" @submit.prevent="emit('submit')">
      <div class="space-y-2">
        <h2 class="text-2xl font-semibold tracking-tight">Подключение к каналу</h2>
        <p class="text-sm leading-relaxed text-muted-foreground">
          Укажите параметры сессии — они сохранятся в адресе после подключения.
        </p>
      </div>

      <div class="grid gap-4 sm:grid-cols-2">
        <label class="grid gap-2 text-sm font-medium">
          <span>Application ID</span>
          <select
            v-model="form.applicationId"
            class="h-10 w-full rounded-md border border-input bg-transparent px-3 text-sm shadow-xs outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
            required
          >
            <option value="staging">staging</option>
          </select>
        </label>

        <label class="grid gap-2 text-sm font-medium">
          <span>Канал</span>
          <Input
            v-model="form.channelName"
            class="h-10"
            autocomplete="off"
            placeholder="private:chat.example"
            required
          />
        </label>

        <label class="grid gap-2 text-sm font-medium">
          <span>Событие</span>
          <Input
            v-model="form.eventName"
            class="h-10"
            autocomplete="off"
            required
          />
        </label>

        <label class="grid gap-2 text-sm font-medium">
          <span>Client ID</span>
          <Input
            v-model="form.clientId"
            class="h-10"
            autocomplete="off"
            required
          />
        </label>

        <label class="grid gap-2 text-sm font-medium sm:col-span-2">
          <span>Ваше имя</span>
          <Input
            v-model="form.displayName"
            class="h-10"
            autocomplete="name"
            maxlength="48"
            placeholder="Как к вам обращаться"
            required
          />
        </label>
      </div>

      <Button class="h-10 w-full" type="submit" :disabled="!canConnect || connecting">
        <LoaderCircleIcon v-if="connecting" class="animate-spin" aria-hidden="true" />
        {{ connecting ? 'Подключаем…' : 'Войти в чат' }}
      </Button>
    </form>
  </div>
</template>
