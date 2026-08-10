<script lang="ts">
  import {
    activityLogs,
    clearActivityLogs,
    consolePanel,
    pushActivityLog,
    setConsoleOpen,
  } from "$lib/activityLog.svelte";
  import type { ActivityLogLevel } from "$lib/types";

  let logContainer: HTMLDivElement | undefined = $state();

  const logs = $derived(activityLogs);
  const errorCount = $derived(logs.filter((entry) => entry.level === "error").length);

  $effect(() => {
    if (!logContainer || !consolePanel.open) return;
    logContainer.scrollTop = logContainer.scrollHeight;
  });

  function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString("ja-JP", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function levelLabel(level: ActivityLogLevel): string {
    switch (level) {
      case "success":
        return "SUCCESS";
      case "warning":
        return "WARN";
      case "error":
        return "ERROR";
      default:
        return "INFO";
    }
  }

  function handleClear() {
    clearActivityLogs();
    pushActivityLog("info", "コンソールをクリアしました");
  }
</script>

{#if consolePanel.open}
  <aside class="console" aria-label="コンソール">
    <header class="console-header">
      <div class="title">
        コンソール
        {#if logs.length > 0}
          <span class="badge">{logs.length}</span>
        {/if}
        {#if errorCount > 0}
          <span class="badge error">{errorCount}</span>
        {/if}
      </div>
      <div class="actions">
        <button class="clear-btn" type="button" onclick={handleClear}>クリア</button>
        <button
          class="close-btn"
          type="button"
          onclick={() => setConsoleOpen(false)}
          aria-label="コンソールを閉じる"
        >
          ×
        </button>
      </div>
    </header>

    <div class="console-body" bind:this={logContainer}>
      {#if logs.length === 0}
        <p class="empty">ログはまだありません</p>
      {:else}
        {#each logs as entry (entry.id)}
          <article class="log-entry level-{entry.level}">
            <div class="log-line">
              <time class="time" datetime={new Date(entry.timestamp).toISOString()}>
                {formatTime(entry.timestamp)}
              </time>
              <span class="level">{levelLabel(entry.level)}</span>
              <span class="message">{entry.message}</span>
            </div>
            {#if entry.detail}
              <pre class="detail">{entry.detail}</pre>
            {/if}
          </article>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .console {
    display: flex;
    flex-direction: column;
    width: 360px;
    flex-shrink: 0;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface-crust);
  }

  .console-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.35rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
    flex-shrink: 0;
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.25rem;
    height: 1.25rem;
    padding: 0 0.35rem;
    border-radius: 999px;
    background: var(--surface-overlay);
    color: var(--text-muted);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .badge.error {
    background: var(--danger-subtle);
    color: var(--danger);
  }

  .clear-btn,
  .close-btn {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
  }

  .close-btn {
    font-size: 0.9rem;
    line-height: 1;
    padding: 0.15rem 0.45rem;
  }

  .clear-btn:hover,
  .close-btn:hover {
    color: var(--text);
    background: var(--surface-hover);
    border-color: var(--surface-active);
  }

  .console-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem 0.75rem;
    font-family: "Cascadia Code", "Consolas", "Menlo", monospace;
    font-size: 0.75rem;
    line-height: 1.45;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .log-entry {
    padding: 0.2rem 0;
    border-bottom: 1px solid color-mix(in srgb, var(--ctp-overlay0) 18%, transparent);
  }

  .log-line {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }

  .time {
    color: var(--text-subtle);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .level {
    width: 4.5rem;
    flex-shrink: 0;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .message {
    color: var(--text);
    word-break: break-word;
  }

  .detail {
    margin: 0.25rem 0 0 0;
    padding: 0.45rem 0.6rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--ctp-crust) 55%, transparent);
    color: var(--text-muted);
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.72rem;
  }

  .level-info .level {
    color: var(--info);
  }

  .level-success .level {
    color: var(--success);
  }

  .level-warning .level {
    color: var(--warning);
  }

  .level-error .level {
    color: var(--danger);
  }

  .level-error .message {
    color: var(--danger-soft);
  }
</style>
