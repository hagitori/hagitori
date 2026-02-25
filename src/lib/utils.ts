/**
 * utility lightweight class name merger.
 * similar to clsx but zero-dependency.
 */
export function cn(
  ...inputs: (string | boolean | undefined | null)[]
): string {
  return inputs.filter(Boolean).join(" ");
}
