<script lang="ts">
interface Props {
	checked: boolean;
	indeterminate?: boolean;
	label: string;
	title?: string;
	variant?: "default" | "overlay";
	onToggle: () => void;
}

let {
	checked,
	indeterminate = false,
	label,
	title,
	variant = "default",
	onToggle,
}: Props = $props();

let inputEl = $state<HTMLInputElement | null>(null);

$effect(() => {
	if (inputEl) {
		inputEl.indeterminate = indeterminate && !checked;
	}
});

function handleClick(event: MouseEvent) {
	event.preventDefault();
	event.stopPropagation();
	onToggle();
}
</script>

<label
  class="selection-checkbox"
  class:checked
  class:indeterminate={!checked && indeterminate}
  class:overlay={variant === "overlay"}
  {title}
>
  <input
    bind:this={inputEl}
    type="checkbox"
    class="native"
    {checked}
    aria-label={label}
    onclick={handleClick}
  />
  <span class="box" aria-hidden="true">
    {#if checked}
      <svg class="mark" viewBox="0 0 16 16" fill="none">
        <path
          d="M3.5 8.2 6.6 11.2 12.5 4.8"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {:else if indeterminate}
      <svg class="mark" viewBox="0 0 16 16" fill="none">
        <path
          d="M4 8h8"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>
    {/if}
  </span>
</label>

<style>
  .selection-checkbox {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    margin: 0;
    cursor: pointer;
    flex-shrink: 0;
    user-select: none;
  }

  .native {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .box {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: 1.5px solid var(--border);
    border-radius: 4px;
    background: var(--surface-overlay);
    color: var(--on-accent);
    transition:
      background 0.12s ease,
      border-color 0.12s ease,
      box-shadow 0.12s ease;
    pointer-events: none;
  }

  .selection-checkbox.overlay .box {
    border-color: color-mix(in srgb, var(--text) 55%, transparent);
    background: color-mix(in srgb, var(--text) 12%, transparent);
  }

  .selection-checkbox:hover .box {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    background: var(--surface-hover);
  }

  .selection-checkbox.overlay:hover .box {
    border-color: color-mix(in srgb, var(--text) 85%, transparent);
    background: color-mix(in srgb, var(--text) 22%, transparent);
  }

  .selection-checkbox.checked .box,
  .selection-checkbox.indeterminate .box {
    border-color: var(--accent);
    background: var(--accent);
  }

  .selection-checkbox.checked:hover .box,
  .selection-checkbox.indeterminate:hover .box {
    border-color: var(--accent-hover);
    background: var(--accent-hover);
  }

  .native:focus-visible + .box {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .mark {
    width: 12px;
    height: 12px;
  }
</style>
