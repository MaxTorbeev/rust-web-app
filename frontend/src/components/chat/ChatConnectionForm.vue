<script setup lang="ts">
import {
  ArrowLeftIcon,
  LoaderCircleIcon,
  MessageCircleIcon,
  MessagesSquareIcon,
  RadioIcon,
  UsersIcon,
} from '@lucide/vue'
import { computed, ref } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { ChatConnectionFormModel } from './types'

defineProps<{
  canConnect: boolean
  connecting: boolean
}>()

const form = defineModel<ChatConnectionFormModel>({ required: true })
const step = ref<1 | 2>(1)

const emit = defineEmits<{
  submit: []
}>()

const canContinue = computed(() =>
  form.value.displayName.trim() !== '' && form.value.clientId.trim() !== '',
)

function showRoomStep(): void {
  if (!canContinue.value) return

  form.value.displayName = form.value.displayName.trim()
  form.value.clientId = form.value.clientId.trim()
  step.value = 2
}
</script>

<template>
  <div class="grid min-h-0 flex-1 md:grid-cols-[1fr_1.05fr]">
    <section class="auth-visual relative hidden min-h-0 flex-col overflow-hidden border-r p-10 md:flex lg:p-14">
      <div class="flex items-center gap-3 text-xl font-semibold tracking-tight">
        <span class="grid size-10 place-items-center rounded-xl bg-primary text-primary-foreground">
          <MessageCircleIcon class="size-5" aria-hidden="true" />
        </span>
        Realtime Chat
      </div>

      <div class="mt-16 max-w-md lg:mt-20">
        <h1 class="text-4xl leading-tight font-semibold tracking-tight lg:text-5xl">
          Общение, которое происходит <span class="text-primary">прямо сейчас</span>
        </h1>
        <p class="mt-5 text-base leading-relaxed text-muted-foreground">
          Быстрые сообщения, присутствие участников и отдельные комнаты в одном простом интерфейсе.
        </p>
      </div>

      <ul class="mt-auto grid gap-5 pt-10">
        <li class="flex items-center gap-4">
          <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
            <MessagesSquareIcon class="size-5" aria-hidden="true" />
          </span>
          <span>
            <strong class="block text-sm font-semibold">Мгновенные сообщения</strong>
            <small class="text-sm text-muted-foreground">Обновления без перезагрузки страницы</small>
          </span>
        </li>
        <li class="flex items-center gap-4">
          <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
            <UsersIcon class="size-5" aria-hidden="true" />
          </span>
          <span>
            <strong class="block text-sm font-semibold">Участники онлайн</strong>
            <small class="text-sm text-muted-foreground">Присутствие и индикатор набора текста</small>
          </span>
        </li>
        <li class="flex items-center gap-4">
          <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
            <RadioIcon class="size-5" aria-hidden="true" />
          </span>
          <span>
            <strong class="block text-sm font-semibold">Отдельные комнаты</strong>
            <small class="text-sm text-muted-foreground">Подключайтесь к существующим или создавайте новые</small>
          </span>
        </li>
      </ul>
    </section>

    <section class="min-h-0 overflow-y-auto bg-card">
      <div class="mx-auto flex min-h-full w-full max-w-lg flex-col justify-center px-6 py-8 sm:px-10 lg:px-14">
        <div class="mb-10 flex items-center justify-between gap-4">
          <div class="flex items-center gap-2 font-semibold md:hidden">
            <MessageCircleIcon class="size-5 text-primary" aria-hidden="true" />
            Realtime Chat
          </div>
          <span class="ml-auto text-xs font-medium text-muted-foreground">Шаг {{ step }} из 2</span>
          <span class="flex gap-1.5" aria-hidden="true">
            <span class="h-1.5 w-8 rounded-full bg-primary" />
            <span class="h-1.5 w-8 rounded-full" :class="step === 2 ? 'bg-primary' : 'bg-secondary'" />
          </span>
        </div>

        <form v-if="step === 1" class="grid gap-6" @submit.prevent="showRoomStep">
          <div class="space-y-2">
            <h2 class="text-3xl font-semibold tracking-tight">Добро пожаловать</h2>
            <p class="text-sm leading-relaxed text-muted-foreground">
              Представьтесь, чтобы другие участники узнали вас в комнате.
            </p>
          </div>

          <label class="grid gap-2 text-sm font-medium">
            <span>Ваше имя</span>
            <Input
              v-model="form.displayName"
              class="h-11"
              autocomplete="name"
              maxlength="48"
              placeholder="Как к вам обращаться"
              autofocus
              required
            />
          </label>

          <label class="grid gap-2 text-sm font-medium">
            <span>Client ID</span>
            <Input
              v-model="form.clientId"
              class="h-11"
              autocomplete="off"
              required
            />
            <small class="font-normal text-muted-foreground">Стабильный идентификатор вашей сессии.</small>
          </label>

          <Button class="h-11 w-full" type="submit" :disabled="!canContinue">
            Продолжить
          </Button>
        </form>

        <form v-else class="grid gap-6" @submit.prevent="emit('submit')">
          <div>
            <Button class="-ml-2 mb-5 px-2" variant="ghost" size="sm" type="button" @click="step = 1">
              <ArrowLeftIcon aria-hidden="true" />
              Назад
            </Button>
            <div class="space-y-2">
              <h2 class="text-3xl font-semibold tracking-tight">Выберите комнату</h2>
              <p class="text-sm leading-relaxed text-muted-foreground">
                Введите существующую комнату или новое имя — она будет создана при подключении.
              </p>
            </div>
          </div>

          <label class="grid gap-2 text-sm font-medium">
            <span>Комната</span>
            <Input
              v-model="form.channelName"
              class="h-11"
              autocomplete="off"
              placeholder="private:chat.example"
              autofocus
              required
            />
          </label>

          <details class="rounded-xl border bg-muted/40 p-4">
            <summary class="cursor-pointer text-sm font-medium">Дополнительные настройки</summary>
            <div class="mt-4 grid gap-4 sm:grid-cols-2">
              <label class="grid gap-2 text-sm font-medium">
                <span>Application ID</span>
                <select
                  v-model="form.applicationId"
                  class="h-10 w-full rounded-md border border-input bg-card px-3 text-sm shadow-xs outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  required
                >
                  <option value="staging">staging</option>
                </select>
              </label>

              <label class="grid gap-2 text-sm font-medium">
                <span>Событие</span>
                <Input v-model="form.eventName" class="h-10" autocomplete="off" required />
              </label>
            </div>
          </details>

          <Button class="h-11 w-full" type="submit" :disabled="!canConnect || connecting">
            <LoaderCircleIcon v-if="connecting" class="animate-spin" aria-hidden="true" />
            {{ connecting ? 'Подключаем…' : 'Войти в комнату' }}
          </Button>
        </form>
      </div>
    </section>
  </div>
</template>
