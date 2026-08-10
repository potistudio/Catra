<slot />

<style>
  /* Catppuccin Mocha */
  :global(:root) {
    --text: #cdd6f4;
    --text-muted: #a6adc8;
    --text-subtle: #6c7086;
    --surface: #1e1e2e;
    --surface-raised: #313244;
    --surface-hover: #45475a;
    --surface-crust: #11111b;
    --border: #45475a;
    --border-subtle: #313244;
    --accent: #cba6f7;
    --accent-subtle: rgba(203, 166, 247, 0.15);
    --danger: #f38ba8;
    --danger-subtle: rgba(243, 139, 168, 0.15);
    --danger-soft: #eba0ac;
    --info: #89b4fa;
    --success: #a6e3a1;
    --warning: #f9e2af;
    --warning-subtle: rgba(249, 226, 175, 0.08);
    --on-accent: #1e1e2e;
    --scrollbar-size: 10px;
    --scrollbar-track: transparent;
    --scrollbar-thumb: #45475a;
    --scrollbar-thumb-hover: #585b70;
    --scrollbar-thumb-active: #cba6f7;
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
</style>
