<slot />

<style>
  :global(:root) {
    --text: #e8e8ec;
    --text-muted: #8b8b96;
    --surface: #1a1a1f;
    --surface-raised: #222228;
    --surface-hover: #2a2a32;
    --border: #33333d;
    --border-subtle: #2a2a32;
    --accent: #6c5ce7;
    --accent-subtle: rgba(108, 92, 231, 0.15);
    --danger: #ff6b6b;
    --danger-subtle: rgba(255, 107, 107, 0.15);
    --scrollbar-size: 10px;
    --scrollbar-track: transparent;
    --scrollbar-thumb: #3a3a46;
    --scrollbar-thumb-hover: #52525e;
    --scrollbar-thumb-active: #6c5ce7;
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
