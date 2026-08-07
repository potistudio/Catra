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
  }
</style>
