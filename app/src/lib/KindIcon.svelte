<script lang="ts">
  /** Outline icon for a device kind, drawn in the current text color. */
  let { kind, size = 20 }: { kind: string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if kind === "mouse"}
    <rect x="6" y="2.5" width="12" height="19" rx="6" />
    <path d="M12 6v4" />
  {:else if kind === "keyboard"}
    <rect x="2" y="6" width="20" height="12" rx="2.5" />
    <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7.5 14h9" />
  {:else if kind === "headset"}
    <path d="M4 15v-3a8 8 0 0 1 16 0v3" />
    <rect x="3" y="14" width="4" height="6" rx="1.5" />
    <rect x="17" y="14" width="4" height="6" rx="1.5" />
  {:else if kind === "gamepad"}
    <path d="M7 8h10a5 5 0 0 1 4.9 6l-.6 2.8a2.4 2.4 0 0 1-4.1 1.1L14.5 16h-5l-2.7 1.9a2.4 2.4 0 0 1-4.1-1.1L2.1 14A5 5 0 0 1 7 8z" />
    <path d="M7 11v3M5.5 12.5h3M16 11.5h.01M18 13.5h.01" />
  {:else if kind === "laptop"}
    <rect x="4" y="5" width="16" height="11" rx="1.5" />
    <path d="M2 19h20" />
  {:else}
    <rect x="5" y="3" width="14" height="18" rx="3" />
    <path d="M12 17h.01" />
  {/if}
</svg>
