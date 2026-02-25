import { create } from "zustand";

type ToastVariant = "success" | "error" | "warning" | "info";

interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
  duration: number;
}

interface ToastStore {
  toasts: Toast[];
  addToast: (
    message: string,
    variant?: ToastVariant,
    duration?: number,
  ) => void;
  removeToast: (id: string) => void;
}

let counter = 0;

const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  addToast: (message, variant = "info", duration = 4000) => {
    const id = `toast-${++counter}`;
    set((state) => ({
      toasts: [...state.toasts, { id, message, variant, duration }],
    }));
    // auto-dismiss
    setTimeout(() => {
      set((state) => ({
        toasts: state.toasts.filter((t) => t.id !== id),
      }));
    }, duration);
  },
  removeToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    })),
}));

export { useToastStore, type Toast, type ToastVariant };
