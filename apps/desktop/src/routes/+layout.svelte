<slot />

<style>
  /* Nord — https://www.nordtheme.com */
  :global(:root) {
    color-scheme: dark;

    --nord0: #2e3440;
    --nord1: #3b4252;
    --nord2: #434c5e;
    --nord3: #4c566a;
    --nord4: #d8dee9;
    --nord5: #e5e9f0;
    --nord6: #eceff4;
    --nord7: #8fbcbb;
    --nord8: #88c0d0;
    --nord9: #81a1c1;
    --nord10: #5e81ac;
    --nord11: #bf616a;
    --nord12: #d08770;
    --nord13: #ebcb8b;
    --nord14: #a3be8c;
    --nord15: #b48ead;

    --text: var(--nord6);
    --text-muted: color-mix(in srgb, var(--nord4) 84%, var(--nord3));
    --text-subtle: color-mix(in srgb, var(--nord4) 58%, var(--nord3));
    --surface: var(--nord0);
    --surface-raised: var(--nord1);
    --surface-overlay: var(--nord2);
    --surface-hover: color-mix(in srgb, var(--nord2) 78%, var(--nord3));
    --surface-active: color-mix(in srgb, var(--nord3) 82%, var(--nord4));
    --surface-selected: color-mix(in srgb, var(--nord10) 42%, var(--nord2));
    --surface-selected-hover: color-mix(in srgb, var(--nord10) 56%, var(--nord2));
    --surface-crust: var(--nord0);
    --border: color-mix(in srgb, var(--nord3) 78%, var(--nord4));
    --border-subtle: color-mix(in srgb, var(--nord2) 72%, var(--nord3));
    --accent: var(--nord8);
    --accent-hover: var(--nord7);
    --accent-subtle: color-mix(in srgb, var(--nord8) 18%, transparent);
    --accent-subtle-hover: color-mix(in srgb, var(--nord8) 28%, transparent);
    --danger: var(--nord11);
    --danger-hover: var(--nord12);
    --danger-subtle: color-mix(in srgb, var(--nord11) 16%, transparent);
    --danger-subtle-hover: color-mix(in srgb, var(--nord11) 26%, transparent);
    --danger-soft: var(--nord12);
    --info: var(--nord9);
    --success: var(--nord14);
    --warning: var(--nord13);
    --warning-subtle: color-mix(in srgb, var(--nord13) 12%, transparent);
    --on-accent: color-mix(in srgb, var(--nord0) 82%, black);
    --overlay: color-mix(in srgb, var(--nord0) 72%, transparent);
    --overlay-strong: color-mix(in srgb, var(--nord0) 82%, transparent);
    --scrollbar-size: 10px;
    --scrollbar-track: transparent;
    --scrollbar-thumb: color-mix(in srgb, var(--nord3) 82%, var(--nord4));
    --scrollbar-thumb-hover: color-mix(in srgb, var(--nord3) 70%, var(--nord4));
    --scrollbar-thumb-active: var(--nord8);
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    font-family: "Inter", system-ui, -apple-system, sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    background: var(--surface);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  }

  :global(*) {
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  }

  :global(*::-webkit-scrollbar) {
    width: var(--scrollbar-size);
    height: var(--scrollbar-size);
  }

  :global(*::-webkit-scrollbar-track) {
    background: var(--scrollbar-track);
  }

  :global(*::-webkit-scrollbar-thumb) {
    background: var(--scrollbar-thumb);
    border: 2px solid transparent;
    border-radius: 999px;
    background-clip: padding-box;
  }

  :global(*::-webkit-scrollbar-thumb:hover) {
    background: var(--scrollbar-thumb-hover);
    border: 2px solid transparent;
    background-clip: padding-box;
  }

  :global(*::-webkit-scrollbar-thumb:active) {
    background: var(--scrollbar-thumb-active);
    border: 2px solid transparent;
    background-clip: padding-box;
  }

  :global(*::-webkit-scrollbar-corner) {
    background: transparent;
  }

  :global(button),
  :global([role="button"]),
  :global(label),
  :global(select),
  :global(a) {
    transition:
      background-color 0.12s ease,
      border-color 0.12s ease,
      color 0.12s ease,
      filter 0.12s ease,
      opacity 0.12s ease;
  }
</style>
