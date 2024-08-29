import { toast } from 'react-toastify';
import { useState } from 'react';

export const raiseErrorToUI = ({ event }: any) => toast(event.error, { type: "error" });

export function getAnimationDelayStyle({ initialDelay, multiple, delay }: {
    initialDelay?: number,
    multiple?: number,
    delay?: number
}) {
    const initial = initialDelay ?? 'var(--initial-delay)';
    const mult = multiple ?? 1;
    const del = delay ?? "var(--delay)";
    return {
        animationDelay: `calc(${initial} + ${mult} * ${del})`,
    };
}

export function useNavDelay(delay = 600) {
    const [isLeaving, setIsLeaving] = useState(false);

    function handleNavigate(nav: () => void) {
        setIsLeaving(true);
        setTimeout(() => nav(), delay);
    }

    return [isLeaving, handleNavigate] as const;
}


export function leaveToTop({
    delay,
    duration,
    timingFunction,
    fillMode
}: {
    delay?: number,
    duration?: number,
    timingFunction?: string,
    fillMode?: "backwards" | "forwards"
} = {}) {
    return {
        animationName: "leave-to-top",
        animationDuration: `${duration ?? 180}ms`,
        animationDelay: `${delay ?? 0}ms`,
        animationFillMode: fillMode ?? "forwards",
        animationTimingFunction: timingFunction ?? "cubic-bezier(0.5, 0, 0.2, 1)",
    }
}

export function enterFromTop({
    delay,
    duration,
    timingFunction,
    fillMode
}: {
    delay?: number,
    duration?: number,
    timingFunction?: string,
    fillMode?: "backwards" | "forwards"
} = {}) {
    return {
        animationName: "enter-from-top",
        animationDuration: `${duration ?? 180}ms`,
        animationDelay: `${delay ?? 0}ms`,
        animationFillMode: fillMode ?? "backwards",
        animationTimingFunction: timingFunction ?? "cubic-bezier(0.5, 0, 0.2, 1)",
    }
}

export function enterFromDown({
    delay,
    duration,
    timingFunction,
    fillMode
}: {
    delay?: number,
    duration?: number,
    timingFunction?: string,
    fillMode?: "backwards" | "forwards"
} = {}) {
    return {
        animationName: "enter-from-down",
        animationDuration: `${duration ?? 180}ms`,
        animationDelay: `${delay ?? 0}ms`,
        animationFillMode: fillMode ?? "backwards",
        animationTimingFunction: timingFunction ?? "cubic-bezier(0.5, 0, 0.2, 1)",
    }
}

export function leaveToDown({
    delay,
    duration,
    timingFunction,
    fillMode
}: {
    delay?: number,
    duration?: number,
    timingFunction?: string,
    fillMode?: "backwards" | "forwards"
} = {}) {
    return {
        animationName: "leave-to-down",
        animationDuration: `${duration ?? 180}ms`,
        animationDelay: `${delay ?? 0}ms`,
        animationFillMode: fillMode ?? "forwards",
        animationTimingFunction: timingFunction ?? "cubic-bezier(0.5, 0, 0.2, 1)",
    }
}

export function leaveToOpacity({
    delay,
    duration,
    timingFunction,
    fillMode
}: {
    delay?: number,
    duration?: number,
    timingFunction?: string,
    fillMode?: "backwards" | "forwards"
} = {}) {
    return {
        animationName: "leave-opacity",
        animationDuration: `${duration ?? 180}ms`,
        animationDelay: `${delay ?? 0}ms`,
        animationFillMode: fillMode ?? "forwards",
        animationTimingFunction: timingFunction ?? "cubic-bezier(0.5, 0, 0.2, 1)",
    }
}

export const LEAVE_TIME = 260;
