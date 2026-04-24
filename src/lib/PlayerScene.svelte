<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import defaultSteveSkinUrl from "../assets/default-steve.png?url";

  export let skinUrl: string | null = null;
  export let sceneAriaLabel = "";

  let container: HTMLDivElement | null = null;
  let canvasEl: HTMLCanvasElement | null = null;
  let viewer: import("skinview3d").SkinViewer | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let currentSkinUrl = "";

  function applySize(): void {
    if (!viewer || !container) {
      return;
    }
    const width = Math.max(container.clientWidth, 1);
    const height = Math.max(container.clientHeight, 1);
    viewer.width = width;
    viewer.height = height;
  }

  async function applySkin(next: string | null): Promise<void> {
    const trimmed = next?.trim() ?? "";
    const effectiveUrl = trimmed || defaultSteveSkinUrl;
    if (!viewer) {
      return;
    }
    if (effectiveUrl === currentSkinUrl) {
      return;
    }
    currentSkinUrl = effectiveUrl;
    await viewer.loadSkin(effectiveUrl, { model: "default" });
  }

  onMount(() => {
    let destroyed = false;

    void (async () => {
      if (!container || !canvasEl) {
        return;
      }

      const skinview3d = await import("skinview3d");
      if (!container || !canvasEl || destroyed) {
        return;
      }

      const nextViewer = new skinview3d.SkinViewer({
        canvas: canvasEl,
        width: Math.max(container.clientWidth, 1),
        height: Math.max(container.clientHeight, 1)
      });

      nextViewer.autoRotate = true;
      nextViewer.autoRotateSpeed = 0.7;

      viewer = nextViewer;
      applySize();

      resizeObserver = new ResizeObserver(() => applySize());
      resizeObserver.observe(container);

      await applySkin(skinUrl);
    })();

    return () => {
      destroyed = true;
    };
  });

  $: void applySkin(skinUrl);

  onDestroy(() => {
    resizeObserver?.disconnect();
    resizeObserver = null;

    if (viewer) {
      viewer.dispose();
      viewer = null;
    }
  });
</script>

<div class="cat-scene" aria-label={sceneAriaLabel || "Player preview"}>
  <div bind:this={container} class="cat-canvas">
    <canvas bind:this={canvasEl}></canvas>
  </div>
</div>

