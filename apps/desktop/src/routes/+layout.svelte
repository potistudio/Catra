<slot />

<style>
  /* Catppuccin Mocha */
  :global(:root) {
    --ctp-rosewater: #f5e0dc;
    --ctp-flamingo: #f2cdcd;
    --ctp-pink: #f5c2e7;
    --ctp-mauve: #cba6f7;
    --ctp-red: #f38ba8;
    --ctp-maroon: #eba0ac;
    --ctp-peach: #fab387;
    --ctp-yellow: #f9e2af;
    --ctp-green: #a6e3a1;
    --ctp-teal: #94e2d5;
    --ctp-sky: #89dceb;
    --ctp-sapphire: #74c7ec;
    --ctp-blue: #89b4fa;
    --ctp-lavender: #b4befe;
    --ctp-text: #cdd6f4;
    --ctp-subtext1: #bac2de;
    --ctp-subtext0: #a6adc8;
    --ctp-overlay2: #9399b2;
    --ctp-overlay1: #7f849c;
    --ctp-overlay0: #6c7086;
    --ctp-surface2: #585b70;
    --ctp-surface1: #45475a;
    --ctp-surface0: #313244;
    --ctp-base: #1e1e2e;
    --ctp-mantle: #181825;
    --ctp-crust: #11111b;

    --text: var(--ctp-text);
    --text-muted: var(--ctp-subtext0);
    --text-subtle: var(--ctp-overlay0);
    --surface: var(--ctp-base);
    --surface-raised: var(--ctp-mantle);
    --surface-overlay: var(--ctp-surface0);
    --surface-hover: var(--ctp-surface0);
    --surface-active: var(--ctp-surface1);
    --surface-crust: var(--ctp-crust);
    --border: var(--ctp-surface0);
    --border-subtle: color-mix(in srgb, var(--ctp-surface0) 55%, var(--ctp-base));
    --accent: var(--ctp-mauve);
    --accent-hover: var(--ctp-lavender);
    --accent-subtle: color-mix(in srgb, var(--ctp-mauve) 18%, transparent);
    --accent-subtle-hover: color-mix(in srgb, var(--ctp-mauve) 28%, transparent);
    --danger: var(--ctp-red);
    --danger-hover: var(--ctp-maroon);
    --danger-subtle: color-mix(in srgb, var(--ctp-red) 16%, transparent);
    --danger-subtle-hover: color-mix(in srgb, var(--ctp-red) 26%, transparent);
    --danger-soft: var(--ctp-maroon);
    --info: var(--ctp-blue);
    --success: var(--ctp-green);
    --warning: var(--ctp-yellow);
    --warning-subtle: color-mix(in srgb, var(--ctp-yellow) 12%, transparent);
    --on-accent: var(--ctp-base);
    --overlay: color-mix(in srgb, var(--ctp-crust) 72%, transparent);
    --overlay-strong: color-mix(in srgb, var(--ctp-crust) 82%, transparent);
    --scrollbar-size: 10px;
    --scrollbar-track: transparent;
    --scrollbar-thumb: var(--ctp-surface1);
    --scrollbar-thumb-hover: var(--ctp-surface2);
    --scrollbar-thumb-active: var(--ctp-mauve);
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
