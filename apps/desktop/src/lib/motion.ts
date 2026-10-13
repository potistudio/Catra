import { cubicOut } from "svelte/easing";
import { fade, slide, type TransitionConfig } from "svelte/transition";

interface MotionOptions {
	duration?: number;
}

interface RevealOptions extends MotionOptions {
	y?: number;
	scale?: number;
}

export function motionDuration(duration: number): number {
	return typeof window !== "undefined" &&
		window.matchMedia("(prefers-reduced-motion: reduce)").matches
		? 0
		: duration;
}

export function fadeMotion(
	node: Element,
	{ duration = 140 }: MotionOptions = {},
): TransitionConfig {
	return fade(node, { duration: motionDuration(duration), easing: cubicOut });
}

export function revealMotion(
	node: Element,
	{ duration = 200, y = 8, scale = 0.98 }: RevealOptions = {},
): TransitionConfig {
	const style = getComputedStyle(node);
	const opacity = Number(style.opacity);
	const transform = style.transform === "none" ? "" : style.transform;

	return {
		duration: motionDuration(duration),
		easing: cubicOut,
		css: (t, u) =>
			`opacity: ${opacity * t}; transform: ${transform} translateY(${y * u}px) scale(${scale + (1 - scale) * t});`,
	};
}

export function panelMotion(node: Element): TransitionConfig {
	return slide(node, {
		axis: "x",
		duration: motionDuration(200),
		easing: cubicOut,
	});
}
