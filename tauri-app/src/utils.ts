import { toast } from 'react-toastify';

export const raiseErrorToUI = ({ event }: any) => toast(event.error, { type: "error" });

export function getAnimationDelayStyle(index: number) {
  return {
    animationDelay: `calc(var(--initial-delay) + ${index} * var(--delay))`,
  };
}
