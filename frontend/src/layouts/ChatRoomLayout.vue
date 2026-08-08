<script setup lang="ts">
defineSlots<{
  members(): unknown
  header(): unknown
  default(): unknown
  composer(): unknown
}>()
</script>

<template>
  <div class="grid min-h-0 flex-1 grid-cols-[15rem_minmax(0,1fr)] max-[760px]:grid-cols-1">
    <aside class="flex min-h-0 min-w-0 flex-col border-r bg-muted p-4 max-[760px]:hidden">
      <slot name="members" />
    </aside>

    <section class="grid min-h-0 min-w-0 grid-rows-[4rem_minmax(0,1fr)_auto]">
      <div class="flex min-w-0 items-center justify-between gap-3 border-b px-4 sm:px-6">
        <slot name="header" />
      </div>

      <div class="min-h-0">
        <slot />
      </div>

      <footer class="border-t bg-card px-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] pt-1 sm:px-5 sm:pb-5">
        <slot name="composer" />
      </footer>
    </section>
  </div>
</template>
