<script lang="ts">
  import { activityLogs, consolePanel, toggleConsole } from "$lib/activityLog.svelte";
  import type { ActivityLogLevel } from "$lib/types";

  const latest = $derived(activityLogs.at(-1) ?? null);
  const errorCount = $derived(activityLogs.filter((entry) => entry.level === "error").length);

  function levelLabel(level: ActivityLogLevel): string {
    switch (level) {
      case "success":
        return "OK";
      case "warning":
        return "WARN";
      case "error":
        return "ERR";
      default:
        return "INFO";
    }
  }
</script>

<div class="status-bar" role="status">
  <button
    class="status-message"
    class:open={consolePanel.open}
    class:level-info={latest?.level === "info"}
    class:level-success={latest?.level === "success"}
    class:level-warning={latest?.level === "warning"}
    class:level-error={latest?.level === "error"}
    type="button"
    onclick={toggleConsole}
    aria-pressed={consolePanel.open}
    title={consolePanel.open ? "コンソールを閉じる" : "コンソールを開く"}
  >
    <span class="console-label">コンソール</span>
    {#if errorCount > 0}
      <span class="badge error">{errorCount}</span>
    {:else if activityLogs.length > 0}
      <span class="badge">{activityLogs.length}</span>
    {/if}
    {#if latest}
      <span class="level">{levelLabel(latest.level)}</span>
      <span class="text">{latest.message}</span>
    {:else}
      <span class="text muted">ログなし</span>
    {/if}
  </button>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    min-height: 22px;
    padding: 0 0.5rem;
    border-top: 1px solid var(--border);
    background: var(--surface-raised);
    flex-shrink: 0;
  }

  .status-message {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    max-width: 100%;
    margin: 0;
    padding: 0.1rem 0.35rem;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.68rem;
    line-height: 1.3;
    cursor: pointer;
    text-align: left;
  }

  .status-message:hover,
  .status-message.open {
    background: var(--surface-hover);
    color: var(--text);
  }

  .console-label {
    flex-shrink: 0;
    font-weight: 600;
    color: var(--text-muted);
  }

  .status-message:hover .console-label,
  .status-message.open .console-label {
    color: var(--text);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1rem;
    height: 1rem;
    padding: 0 0.28rem;
    border-radius: 999px;
    background: var(--surface-hover);
    color: var(--text-muted);
    font-size: 0.62rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .badge.error {
    background: var(--danger-subtle);
    color: var(--danger);
  }

  .level {
    flex-shrink: 0;
    font-weight: 700;
    letter-spacing: 0.02em;
    font-variant-numeric: tabular-nums;
  }

  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text.muted {
    color: var(--text-muted);
  }

  .level-info .level {
    color: #7aa2f7;
  }

  .level-success .level {
    color: #9ece6a;
  }

  .level-warning .level {
    color: #e0af68;
  }

  .level-error .level {
    color: var(--danger);
  }

  .level-error .text {
    color: #ffb4b4;
  }
</style>
