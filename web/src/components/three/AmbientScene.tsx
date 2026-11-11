import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { Float } from "@react-three/drei";
import { useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";

/**
 * Ambient 3D backdrop: translucent "poster" panes drifting through soft light. Used behind the
 * sign-in card and the empty home state. Rendered lazily and only when motion is allowed.
 *
 * It is a soft background, so it is drawn as cheaply as it can be (design.md Q155): frames on
 * demand at 30 a second, none while it's scrolled out of view or the tab is hidden, at 1× pixel
 * density without antialiasing, and with plain translucent panes (a physical "transmission"
 * material renders the whole scene a second time). A software renderer, or a GPU that can't keep
 * up in the first seconds, gets the plain gradient instead (`onSlow`).
 */

/** Frames a second the scene asks for; the drift is slow, so more buys nothing. */
const FPS = 30;
/** Seconds of frames measured before deciding the GPU keeps up, after a second to settle. */
const MEASURE = { settle: 1000, window: 3000 } as const;

const PANE_COLOURS = ["#7c6cff", "#4fd1ff", "#ffb35c", "#ff5ea8", "#5cf0b8"];

interface PaneProps {
  position: [number, number, number];
  rotation: [number, number, number];
  colour: string;
  scale: number;
  speed: number;
  /** Optional cover image; the pane becomes a floating poster when it loads. */
  image?: string;
}

/** Loads a same-origin image into a texture; resolves to null when it cannot be shown. */
function useCoverTexture(url: string | undefined): THREE.Texture | null {
  const [texture, setTexture] = useState<THREE.Texture | null>(null);
  useEffect(() => {
    if (!url) return;
    let cancelled = false;
    const loader = new THREE.TextureLoader();
    if (!url.startsWith("data:")) loader.setCrossOrigin("use-credentials");
    loader.load(
      url,
      (loaded) => {
        if (cancelled) return;
        loaded.colorSpace = THREE.SRGBColorSpace;
        setTexture(loaded);
      },
      undefined,
      () => setTexture(null),
    );
    return () => {
      cancelled = true;
    };
  }, [url]);
  return texture;
}

function Pane({ position, rotation, colour, scale, speed, image }: PaneProps) {
  const mesh = useRef<THREE.Mesh>(null);
  const texture = useCoverTexture(image);
  useFrame(({ clock }) => {
    if (!mesh.current) return;
    const t = clock.getElapsedTime() * speed;
    mesh.current.rotation.y = rotation[1] + Math.sin(t * 0.35) * 0.25;
    mesh.current.rotation.x = rotation[0] + Math.cos(t * 0.28) * 0.12;
  });
  return (
    <Float speed={speed * 1.4} rotationIntensity={0.15} floatIntensity={0.9} floatingRange={[-0.25, 0.25]}>
      <mesh ref={mesh} position={position} rotation={rotation} scale={[scale * 0.68, scale, 1]}>
        <planeGeometry args={[1, 1, 1, 1]} />
        {texture ? (
          <meshStandardMaterial map={texture} roughness={0.35} metalness={0.05} transparent opacity={0.92} side={THREE.DoubleSide} />
        ) : (
          <meshStandardMaterial color={colour} transparent opacity={0.5} roughness={0.25} metalness={0.1} side={THREE.DoubleSide} />
        )}
      </mesh>
    </Float>
  );
}

function Panes({ count, images }: { count: number; images: string[] }) {
  const panes = useMemo<PaneProps[]>(() => {
    const random = mulberry32(1337);
    return Array.from({ length: count }, (_, index) => ({
      position: [(random() - 0.5) * 9, (random() - 0.5) * 5, -2 - random() * 5],
      rotation: [(random() - 0.5) * 0.6, (random() - 0.5) * 1.2, (random() - 0.5) * 0.3],
      colour: PANE_COLOURS[index % PANE_COLOURS.length]!,
      scale: 1.2 + random() * 1.6,
      speed: 0.35 + random() * 0.5,
      image: images[index % Math.max(1, images.length)],
    }));
  }, [count, images]);
  return (
    <>
      {panes.map((pane, index) => (
        <Pane key={index} {...pane} />
      ))}
    </>
  );
}

function CameraDrift() {
  useFrame(({ camera, clock, pointer }) => {
    const t = clock.getElapsedTime();
    camera.position.x += (pointer.x * 0.6 + Math.sin(t * 0.12) * 0.3 - camera.position.x) * 0.02;
    camera.position.y += (pointer.y * 0.4 + Math.cos(t * 0.1) * 0.2 - camera.position.y) * 0.02;
    camera.lookAt(0, 0, -3);
  });
  return null;
}

/**
 * Drives the scene's frames itself (the canvas renders on demand): one frame per 1/FPS while the
 * canvas is on screen and the tab is visible. It also gives up on GPUs that can't keep up.
 */
function Pacer({ onSlow }: { onSlow: () => void }) {
  const { invalidate, gl } = useThree();
  useEffect(() => {
    const context = gl.getContext();
    const info = context.getExtension("WEBGL_debug_renderer_info");
    const renderer = info ? String(context.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
    if (/swiftshader|llvmpipe|software|basic render/i.test(renderer)) {
      onSlow();
      return;
    }
    let onScreen = true;
    const observer = new IntersectionObserver(([entry]) => {
      onScreen = Boolean(entry?.isIntersecting);
    });
    observer.observe(gl.domElement);
    const started = performance.now();
    let last = 0;
    let measured = 0;
    let decided = false;
    let frame = requestAnimationFrame(function tick(now) {
      frame = requestAnimationFrame(tick);
      if (!onScreen || document.hidden || now - last < 1000 / FPS - 2) return;
      last = now;
      invalidate();
      const elapsed = now - started;
      if (decided || elapsed < MEASURE.settle) return;
      measured += 1;
      if (elapsed < MEASURE.settle + MEASURE.window) return;
      decided = true;
      // Well under the frame rate it asked for: the GPU is struggling, and the page with it.
      if (measured < (MEASURE.window / 1000) * FPS * 0.6) onSlow();
    });
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [gl, invalidate, onSlow]);
  return null;
}

/** Deterministic PRNG so the scene looks the same on every load. */
function mulberry32(seed: number) {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export default function AmbientScene({ density = 11, images = [], onSlow }: { density?: number; images?: string[]; onSlow: () => void }) {
  return (
    <Canvas
      dpr={1}
      frameloop="demand"
      camera={{ position: [0, 0, 6], fov: 45 }}
      gl={{ antialias: false, alpha: true, powerPreference: "low-power" }}
      className="!absolute inset-0"
      aria-hidden
    >
      <ambientLight intensity={0.6} />
      <directionalLight position={[4, 6, 5]} intensity={1.4} color="#fff2dd" />
      <pointLight position={[-6, -3, 2]} intensity={12} color="#7c6cff" distance={14} />
      <pointLight position={[6, 2, 1]} intensity={9} color="#4fd1ff" distance={12} />
      <Panes count={density} images={images} />
      <CameraDrift />
      <Pacer onSlow={onSlow} />
    </Canvas>
  );
}
