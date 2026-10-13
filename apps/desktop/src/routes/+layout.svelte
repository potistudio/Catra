<slot />

<style>
  :global(:root) {
    color-scheme: dark;
    --text: #f2f2f4;
    --text-muted: #b8b8be;
    --text-subtle: #929298;
    --surface: #202022;
    --surface-raised: #29292c;
    --surface-overlay: #343437;
    --surface-hover: #353538;
    --surface-active: #55555b;
    --surface-selected: #424248;
    --surface-selected-hover: #49494f;
    --surface-crust: #1b1b1d;
    --surface-input: #1c1c1e;
    --surface-chrome: linear-gradient(180deg, #333336, #2b2b2e);
    --surface-sidebar: linear-gradient(180deg, #2c2c2f, #262628);
    --border: rgb(255 255 255 / 12%);
    --border-subtle: rgb(255 255 255 / 6%);
    --accent: #d0d0d6;
    --accent-hover: #e5e5ea;
    --accent-subtle: rgb(208 208 214 / 12%);
    --accent-subtle-hover: rgb(208 208 214 / 20%);
    --danger: #ff8580;
    --danger-hover: #ffa39e;
    --danger-subtle: rgb(255 133 128 / 12%);
    --danger-subtle-hover: rgb(255 133 128 / 20%);
    --danger-soft: #ffb1a5;
    --info: #a8c7ed;
    --success: #8fd5a3;
    --warning: #edcb86;
    --warning-subtle: rgb(237 203 134 / 12%);
    --on-accent: #242426;
    --overlay: rgb(0 0 0 / 48%);
    --overlay-strong: rgb(0 0 0 / 72%);
    --radius-control: 7px;
    --radius-panel: 12px;
    --radius-dialog: 16px;
    --shadow-control: inset 0 1px 0 rgb(255 255 255 / 7%), 0 1px 2px rgb(0 0 0 / 18%);
    --shadow-inset: inset 0 1px 3px rgb(0 0 0 / 20%);
    --shadow-floating: 0 8px 24px rgb(0 0 0 / 28%), inset 0 1px 0 rgb(255 255 255 / 6%);
    --shadow-dialog: 0 24px 80px rgb(0 0 0 / 48%), inset 0 1px 0 rgb(255 255 255 / 8%);
    --scrollbar-size: 9px;
    --scrollbar-track: transparent;
    --scrollbar-thumb: #5c5c62;
    --scrollbar-thumb-hover: #77777d;
    --scrollbar-thumb-active: #929298;
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI Variable", "Segoe UI", "Noto Sans JP", sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    background: var(--surface);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
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

  :global(::selection) {
    color: var(--text);
    background: var(--surface-selected);
  }

  :global(:where(button, input, select, textarea)) {
    font-family: inherit;
    accent-color: var(--accent);
  }

  :global(:where(input, textarea)::placeholder) {
    color: var(--text-subtle);
    opacity: 1;
  }

  :global(:where(button, [role="button"], input, select, textarea, a):focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  :global(button),
  :global([role="button"]),
  :global(label),
  :global(select),
  :global(a) {
    transition:
      background-color 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease,
      box-shadow 0.15s ease,
      filter 0.15s ease,
      opacity 0.15s ease;
  }

  @media (prefers-reduced-motion: reduce) {
    :global(*, *::before, *::after) {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
      scroll-behavior: auto !important;
    }
  }
</style>
