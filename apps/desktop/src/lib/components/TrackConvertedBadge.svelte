<script lang="ts">
import { audioFormatLabel } from "$lib/format";

interface Props {
	path: string;
}

let { path }: Props = $props();

const format = $derived(audioFormatLabel(path));
const _label = $derived(format ? `変換済み · ${format}` : "変換済み");
</script>

<span class="converted-badge" title={label} aria-label={label}>変換済み</span>

<style>
  .converted-badge {
    flex-shrink: 0;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    background: var(--overlay-strong);
    color: var(--text-muted);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    line-height: 1.3;
    white-space: nowrap;
  }
</style>
