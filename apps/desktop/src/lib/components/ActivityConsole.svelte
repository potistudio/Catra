<script lang="ts">
  import {
    activityLogs,
    clearActivityLogs,
    pushActivityLog,
  } from "$lib/activityLog.svelte";
  import type { ActivityLogLevel } from "$lib/types";

  let isExpanded = $state(true);
  let logContainer: HTMLDivElement | undefined = $state();

  const logs = $derived(activityLogs);

  const errorCount = $derived(logs.filter((entry) => entry.level === "error").length);

  $effect(() => {
    if (!logContainer || !isExpanded) return;
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

<section class="console" class:collapsed={!isExpanded}>
  <header class="console-header">
    <button
      class="toggle"
      type="button"
      onclick={() => (isExpanded = !isExpanded)}
      aria-expanded={isExpanded}
    >
      <span class="chevron" class:open={isExpanded}>▸</span>
      コンソール
      {#if logs.length > 0}
        <span class="badge">{logs.length}</span>
      {/if}
      {#if errorCount > 0}
        <span class="badge error">{errorCount}</span>
      {/if}
    </button>

    {#if isExpanded}
      <button class="clear-btn" type="button" onclick={handleClear}>クリア</button>
    {/if}
  </header>

  {#if isExpanded}
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
  {:else if logs.length > 0}
    <p class="preview">
      {logs[logs.length - 1]?.message}
    </p>
  {/if}
</section>

<style>
  .console {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
    background: #141418;
    min-height: 0;
  }

  .console.collapsed {
    flex: 0 0 auto;
  }

  .console:not(.collapsed) {
    flex: 0 0 220px;
  }

  .console-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.35rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0.15rem 0;
  }

  .chevron {
    display: inline-block;
    transition: transform 0.15s ease;
    color: var(--text-muted);
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.25rem;
    height: 1.25rem;
    padding: 0 0.35rem;
    border-radius: 999px;
    background: var(--surface-hover);
    color: var(--text-muted);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .badge.error {
    background: var(--danger-subtle);
    color: var(--danger);
  }

  .clear-btn {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
  }

  .clear-btn:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .console-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem 0.75rem;
    font-family: "Cascadia Code", "Consolas", "Menlo", monospace;
    font-size: 0.75rem;
    line-height: 1.45;
  }

  .empty,
  .preview {
    margin: 0;
    padding: 0.35rem 0.75rem 0.5rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .log-entry {
    padding: 0.2rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .log-line {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }

  .time {
    color: #6b6b78;
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
    margin: 0.25rem 0 0 7.35rem;
    padding: 0.45rem 0.6rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.25);
    color: #b8b8c4;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.72rem;
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

  .level-error .message {
    color: #ffb4b4;
  }
</style>
