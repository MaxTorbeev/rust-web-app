<script setup lang="ts">
withDefaults(defineProps<{
  showHeader?: boolean
}>(), {
  showHeader: true,
})

defineSlots<{
  header?(): unknown
  default(): unknown
  overlay?(): unknown
}>()
</script>

<template>
  <main class="grid min-h-dvh place-items-center bg-background p-0 sm:p-6">
    <section class="relative flex h-dvh w-full max-w-280 flex-col overflow-hidden border-border bg-card shadow-sm sm:h-[min(48rem,calc(100dvh-3rem))] sm:rounded-2xl sm:border">
      <header v-if="showHeader" class="flex min-h-18 flex-wrap items-center justify-between gap-3 border-b px-4 py-3 sm:px-6">
        <slot name="header" />
      </header>
      <slot />
      <slot name="overlay" />
    </section>
  </main>
</template>
