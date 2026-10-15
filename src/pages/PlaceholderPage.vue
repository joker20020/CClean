<script setup lang="ts">
defineProps<{ title: string; milestone: string }>();
</script>

<template>
  <div class="panel placeholder">
    <div class="ph-icon">🚧</div>
    <div class="ph-title">{{ title }}</div>
    <div class="muted">{{ $t("common.comingSoon") }}</div>
    <div class="ph-milestone">{{ milestone }}</div>
    <div class="muted" style="margin-top: 18px">
      {{ $t("common.m1Ready") }}
    </div>
  </div>
</template>

<style scoped>
.placeholder {
  text-align: center;
  padding: 70px 20px;
}
.ph-icon {
  font-size: 40px;
}
.ph-title {
  font-size: 18px;
  font-weight: 700;
  margin: 14px 0 8px;
}
.ph-milestone {
  margin-top: 14px;
  color: #2563eb;
}
</style>
