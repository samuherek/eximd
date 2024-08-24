import { toast } from 'react-toastify';

export const raiseErrorToUI = ({ event }: any) => toast(event.error, { type: "error" });
