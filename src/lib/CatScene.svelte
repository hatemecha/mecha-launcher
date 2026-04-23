<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  import blackCatModelUrl from "../../minecraft-black-cat/source/model.gltf?url";
  import whiteCatModelUrl from "../../minecraft-white-cat/source/model.gltf?url";
  import type {
    Group,
    Material,
    Mesh,
    Object3D,
    PerspectiveCamera,
    Scene,
    Texture,
    WebGLRenderer
  } from "three";
  import type { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";

  type ThemeMode = "light" | "dark";

  export let themeMode: ThemeMode = "dark";

  let container: HTMLDivElement | null = null;
  let three: typeof import("three") | null = null;
  let renderer: WebGLRenderer | null = null;
  let scene: Scene | null = null;
  let camera: PerspectiveCamera | null = null;
  let pivot: Group | null = null;
  let loader: GLTFLoader | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let intersectionObserver: IntersectionObserver | null = null;
  let animationFrameId: number | null = null;
  let currentModel: Object3D | null = null;
  let currentModelUrl = "";
  let modelLoadToken = 0;
  let isMounted = false;
  let isDestroyed = false;
  let isSceneVisible = true;
  let lastFrameTime = 0;

  $: modelUrl = themeMode === "light" ? blackCatModelUrl : whiteCatModelUrl;
  $: modelLabel = themeMode === "light" ? "Black cat" : "White cat";

  $: if (isMounted && modelUrl !== currentModelUrl) {
    void loadModel(modelUrl);
  }

  function getCanvasSize(): { width: number; height: number } {
    const width = Math.max(container?.clientWidth ?? 0, 1);
    const height = Math.max(container?.clientHeight ?? 0, 1);
    return { width, height };
  }

  function resizeRenderer(): void {
    if (!renderer || !camera) {
      return;
    }

    const { width, height } = getCanvasSize();
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
    renderScene();
  }

  function renderScene(): void {
    if (renderer && scene && camera) {
      renderer.render(scene, camera);
    }
  }

  function scheduleFrame(): void {
    if (!isSceneVisible || animationFrameId !== null) {
      return;
    }

    animationFrameId = requestAnimationFrame(animate);
  }

  function animate(frameTime: number): void {
    animationFrameId = null;

    const deltaSeconds = Math.min((frameTime - lastFrameTime) / 1000 || 0, 0.05);
    lastFrameTime = frameTime;

    if (pivot) {
      pivot.rotation.y += deltaSeconds * 0.7;
    }

    renderScene();
    scheduleFrame();
  }

  function disposeMaterial(material: Material): void {
    const textureKeys = [
      "map",
      "alphaMap",
      "aoMap",
      "bumpMap",
      "displacementMap",
      "emissiveMap",
      "envMap",
      "lightMap",
      "metalnessMap",
      "normalMap",
      "roughnessMap"
    ] as const;

    const materialRecord = material as Material & Record<string, unknown>;

    for (const key of textureKeys) {
      const texture = materialRecord[key];
      if (texture && typeof texture === "object" && "dispose" in texture) {
        (texture as Texture).dispose();
      }
    }

    material.dispose();
  }

  function disposeObject(root: Object3D): void {
    root.traverse((child) => {
      const mesh = child as Mesh;

      if (mesh.geometry) {
        mesh.geometry.dispose();
      }

      if (!mesh.material) {
        return;
      }

      const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
      for (const material of materials) {
        disposeMaterial(material);
      }
    });
  }

  function prepareModel(model: Object3D): Object3D {
    const threeModule = three;
    if (!threeModule) {
      return model;
    }

    model.traverse((child) => {
      const mesh = child as Mesh;

      if (!mesh.isMesh) {
        return;
      }

      mesh.frustumCulled = true;

      const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
      for (const material of materials) {
        material.side = threeModule.DoubleSide;
        material.transparent = false;
        material.depthWrite = true;
        material.depthTest = true;
        material.needsUpdate = true;
      }

      mesh.geometry.computeBoundingBox();
      mesh.geometry.computeBoundingSphere();
    });

    return model;
  }

  function centerModel(model: Object3D): void {
    const threeModule = three;
    if (!threeModule || !camera || !pivot) {
      return;
    }

    const box = new threeModule.Box3().setFromObject(model);
    const center = box.getCenter(new threeModule.Vector3());
    const size = box.getSize(new threeModule.Vector3());
    const maxDimension = Math.max(size.x, size.y, size.z, 1);

    model.position.sub(center);
    pivot.position.set(0, 0, 0);
    pivot.rotation.set(0.08, 0, 0);

    const fovRadians = threeModule.MathUtils.degToRad(camera.fov);
    const cameraZ = Math.abs(maxDimension / 2 / Math.tan(fovRadians / 2)) * 2.35;
    camera.position.set(0, maxDimension * 0.1, cameraZ);
    camera.lookAt(0, 0, 0);
  }

  async function loadModel(nextModelUrl: string): Promise<void> {
    if (!loader || !pivot) {
      return;
    }

    const loadToken = ++modelLoadToken;
    currentModelUrl = nextModelUrl;

    if (currentModel) {
      pivot.remove(currentModel);
      disposeObject(currentModel);
      currentModel = null;
    }

    try {
      const gltf = await loader.loadAsync(nextModelUrl);

      if (loadToken !== modelLoadToken || currentModelUrl !== nextModelUrl || !pivot) {
        disposeObject(gltf.scene);
        return;
      }

      const model = prepareModel(gltf.scene);
      centerModel(model);
      pivot.add(model);
      currentModel = model;
      renderScene();
    } catch (error) {
      console.error("Failed to load cat model:", error);
    }
  }

  async function initializeScene(): Promise<void> {
    if (!container) {
      return;
    }

    const [threeModule, loaderModule] = await Promise.all([
      import("three"),
      import("three/examples/jsm/loaders/GLTFLoader.js")
    ]);

    if (!container || isDestroyed) {
      return;
    }

    three = threeModule;
    threeModule.Cache.enabled = true;

    const { width, height } = getCanvasSize();
    scene = new threeModule.Scene();
    pivot = new threeModule.Group();
    scene.add(pivot);

    camera = new threeModule.PerspectiveCamera(45, width / height, 0.1, 100);
    loader = new loaderModule.GLTFLoader();

    renderer = new threeModule.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: "low-power"
    });
    renderer.outputColorSpace = threeModule.SRGBColorSpace;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.75));
    renderer.setSize(width, height, false);
    renderer.setClearColor(0x000000, 0);
    container.appendChild(renderer.domElement);

    scene.add(new threeModule.AmbientLight(0xffffff, 1.25));

    const keyLight = new threeModule.DirectionalLight(0xffffff, 1.45);
    keyLight.position.set(4, 6, 5);
    scene.add(keyLight);

    const rimLight = new threeModule.DirectionalLight(0xffffff, 0.5);
    rimLight.position.set(-4, 3, -3);
    scene.add(rimLight);

    resizeObserver = new ResizeObserver(resizeRenderer);
    resizeObserver.observe(container);

    intersectionObserver = new IntersectionObserver(([entry]) => {
      isSceneVisible = entry?.isIntersecting ?? true;
      if (isSceneVisible) {
        lastFrameTime = performance.now();
        scheduleFrame();
      } else if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
      }
    });
    intersectionObserver.observe(container);

    isMounted = true;
    void loadModel(modelUrl);
    lastFrameTime = performance.now();
    scheduleFrame();
  }

  onMount(() => {
    isDestroyed = false;
    void initializeScene();

    return () => {
      isMounted = false;
    };
  });

  onDestroy(() => {
    isDestroyed = true;

    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }

    resizeObserver?.disconnect();
    intersectionObserver?.disconnect();

    if (currentModel && pivot) {
      pivot.remove(currentModel);
      disposeObject(currentModel);
      currentModel = null;
    }

    renderer?.dispose();
    renderer?.domElement.remove();

    renderer = null;
    scene = null;
    camera = null;
    pivot = null;
    loader = null;
    three = null;
  });
</script>

<div class="cat-scene" aria-label={`${modelLabel} rotating preview`}>
  <div bind:this={container} class="cat-canvas"></div>
  <div class="cat-meta">
    <span>{modelLabel}</span>
    <span>Theme inverse model</span>
  </div>
</div>
