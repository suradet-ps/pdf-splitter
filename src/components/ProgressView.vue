<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  /* Progress percentage (0–100) */
  percent: number;
  /* Pages completed so far (1-based) */
  current: number;
  /* Total pages to process */
  total: number;
  /* Filename of the most-recently written output page */
  currentFile: string;
  /* Basename of the source PDF file */
  fileName: string;
}>();

const clampedPercent = computed<number>(() => Math.min(100, Math.max(0, props.percent)));

const fillTransform = computed<string>(() => `scaleX(${clampedPercent.value / 100})`);

const fractionLabel = computed<string>(() =>
  props.total > 0 ? `${props.current} / ${props.total}` : '…',
);

const isStarting = computed<boolean>(() => props.current === 0);

const isFinalising = computed<boolean>(() => clampedPercent.value >= 100 && !isStarting.value);

const MAX_DOTS = 20;
const stepDots = computed<boolean[]>(() => {
  if (props.total <= 0) return [];
  const count = Math.min(props.total, MAX_DOTS);
  return Array.from({ length: count }, (_, i) => {
    const threshold = Math.round(((i + 1) / count) * props.total);
    return props.current >= threshold;
  });
});
</script>

<template>
<div class="progress-view" role="status" aria-live="polite" aria-label="Splitting PDF…">

    <!-- Header row -->
    <div class="progress-view__header">

        <!-- Animated status indicator -->
        <div class="progress-view__indicator" aria-hidden="true">
            <template v-if="isFinalising">
                <!-- Checkmark when finalising -->
                <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                    <circle cx="12" cy="12" r="11" stroke="currentColor" stroke-width="1.5" opacity="0.3"/>
                    <path fill-rule="evenodd" clip-rule="evenodd"
                        d="M17.09 8.47a.75.75 0 0 1 0 1.06l-5.5 5.5a.75.75 0 0 1-1.06 0l-2.5-2.5a.75.75 0 1 1 1.06-1.06l1.97 1.97 4.97-4.97a.75.75 0 0 1 1.06 0Z"
                        fill="currentColor"/>
                </svg>
            </template>
            <template v-else>
                <!-- Spinning arc when processing -->
                <svg class="progress-view__spinner animate-spin" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                    <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5" opacity="0.15"/>
                    <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
            </template>
        </div>

        <!-- Title + status -->
        <div class="progress-view__title-group">
            <h2 class="progress-view__title">
                <template v-if="isFinalising">Finalising…</template>
                <template v-else>Splitting PDF</template>
            </h2>
            <p class="progress-view__filename truncate" :title="fileName">{{ fileName }}</p>
            <p class="progress-view__status">
                <template v-if="isStarting">
                    <span class="animate-pulse">Preparing pages…</span>
                </template>
                <template v-else-if="isFinalising">
                    <span>Writing output files…</span>
                </template>
                <template v-else>
                    <span>Processing page <strong>{{ current }}</strong> of <strong>{{ total }}</strong></span>
                </template>
            </p>
        </div>

        <!-- Percent badge -->
        <div
            class="progress-view__pct"
            :class="{ 'progress-view__pct--finalising': isFinalising }"
            aria-hidden="true"
        >
            <span class="progress-view__pct-number">
                {{ isStarting ? '0' : clampedPercent }}
            </span>
            <span class="progress-view__pct-unit">%</span>
        </div>

    </div>

    <!-- Progress bar area -->
    <div class="progress-section">

        <!-- Fraction label -->
        <div class="progress-section__label" aria-hidden="true">
            <span class="progress-section__fraction">{{ fractionLabel }}</span>
            <span class="progress-section__unit">pages</span>
        </div>

        <!-- Track + fill -->
        <div
            class="progress-track"
            role="progressbar"
            :aria-valuenow="clampedPercent"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-label="`${clampedPercent}% complete`"
        >
            <div class="progress-fill" :style="{ transform: fillTransform }" />
        </div>

        <!-- Step dots -->
        <div v-if="stepDots.length > 1" class="step-dots" aria-hidden="true">
            <span
                v-for="(done, i) in stepDots"
                :key="i"
                class="step-dot"
                :class="{ 'step-dot--done': done }"
            />
        </div>

    </div>

    <!-- Output stream -->
    <div class="output-stream">

        <Transition name="fade" mode="out-in">
            <div v-if="currentFile" key="file" class="output-line output-line--ok">
                <svg viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="12" height="12" aria-hidden="true" class="output-line__check-icon">
                    <path fill-rule="evenodd" clip-rule="evenodd"
                        d="M11.78 3.22a.75.75 0 0 1 0 1.06l-5.5 5.5a.75.75 0 0 1-1.06 0L2.72 7.28a.75.75 0 1 1 1.06-1.06l2.47 2.47 4.97-4.97a.75.75 0 0 1 1.06 0Z"
                        fill="currentColor"/>
                </svg>
                <span class="output-line__text truncate">{{ currentFile }}</span>
            </div>
            <div v-else-if="isStarting" key="pending" class="output-line output-line--pending">
                <span class="output-line__dot animate-pulse" aria-hidden="true" />
                <span class="output-line__text">Waiting for first page…</span>
            </div>
        </Transition>

    </div>

</div>
</template>

<style scoped>
.progress-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-7);
    width: 100%;
    padding: var(--space-2) 0;
}

/* ─── Header ─── */

.progress-view__header {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
}

/* Status icon (spinner / check) */
.progress-view__indicator {
    flex-shrink: 0;
    width: var(--size-icon-status);
    height: var(--size-icon-status);
    color: var(--color-accent);
    margin-top: var(--space-1);
    filter: drop-shadow(0 0 6px var(--color-accent-glow));
}

/* Title group */
.progress-view__title-group {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
}

.progress-view__title {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    color: var(--color-text-primary);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
}

.progress-view__filename {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
    line-height: var(--leading-normal);
    max-width: var(--size-filename-max);
}

.progress-view__status {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-tertiary);
    line-height: var(--leading-normal);
}

.progress-view__status strong {
    color: var(--color-text-secondary);
    font-weight: var(--weight-semibold);
}

/* Percent badge */
.progress-view__pct {
    flex-shrink: 0;
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    background: var(--color-surface-raised);
    border: var(--border-thin) solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-pct-y) var(--space-pct-x);
    min-width: var(--size-min-w-pct);
    justify-content: center;
    transition:
        color var(--duration-normal) var(--ease-out),
        border-color var(--duration-normal) var(--ease-out);
}

.progress-view__pct--finalising {
    color: var(--color-accent);
    border-color: rgba(var(--color-accent-rgb), var(--opacity-busy));
}

.progress-view__pct-number {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-semibold);
    color: inherit;
    line-height: var(--leading-tight);
}

.progress-view__pct-unit {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    color: var(--color-text-quaternary);
    line-height: var(--leading-tight);
}

.progress-view__pct--finalising .progress-view__pct-number,
.progress-view__pct--finalising .progress-view__pct-unit {
    color: var(--color-accent);
}

/* ─── Progress section ─── */

.progress-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
}

.progress-section__label {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
}

.progress-section__fraction {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
    color: var(--color-text-secondary);
    line-height: var(--leading-tight);
}

.progress-section__unit {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-quaternary);
    line-height: var(--leading-tight);
}

/* Track + fill — from global .progress-track / .progress-fill styles */
.progress-section .progress-track {
    height: var(--size-progress-track);
    will-change: contents;
}

/* Step dots */
.step-dots {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-1);
}

.step-dot {
    flex: 1;
    height: var(--size-step-dot);
    border-radius: var(--size-step-dot-radius);
    background: var(--color-border);
    transition: background-color var(--duration-normal) var(--ease-out);
}

.step-dot--done {
    background: var(--color-accent);
    opacity: var(--opacity-busy);
}

/* ─── Output stream ─── */

.output-stream {
    min-height: var(--size-pct-min-h);
    display: flex;
    flex-direction: column;
}

.output-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    padding: var(--space-output-line-y) var(--space-output-line-x);
    border-radius: var(--radius-md);
    border-left: var(--border-thick) solid transparent;
}

.output-line--ok {
    color: var(--color-success-text);
    background: var(--color-success-subtle);
    border-left-color: var(--color-accent);
}

.output-line--pending {
    color: var(--color-text-quaternary);
    border-left-color: var(--color-border);
    background: var(--color-surface-inset);
}

.output-line__check-icon {
    flex-shrink: 0;
    color: var(--color-accent);
}

.output-line__dot {
    flex-shrink: 0;
    width: var(--size-dot-sm);
    height: var(--size-dot-sm);
    border-radius: 50%;
    background: var(--color-text-quaternary);
    display: inline-block;
}

.output-line__text {
    flex: 1;
    min-width: 0;
    color: inherit;
}
</style>