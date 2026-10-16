<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    class: className,
    variant = "default",
    size = "default",
    children,
    ...restProps
  }: HTMLButtonAttributes & {
    variant?: "default" | "secondary" | "outline" | "ghost";
    size?: "default" | "sm" | "lg" | "icon";
  } = $props();

  const variants = {
    default:
      "bg-primary text-primary-foreground shadow-none hover:bg-primary/90",
    secondary:
      "bg-secondary text-secondary-foreground shadow-none hover:bg-secondary/80",
    outline:
      "bg-background shadow-xs hover:bg-accent hover:text-accent-foreground",
    ghost: "shadow-none hover:bg-accent hover:text-accent-foreground",
  };
  const sizes = {
    default: "h-9 px-4 py-2",
    sm: "h-8 px-3 text-xs",
    lg: "h-10 px-6",
    icon: "size-9",
  };
</script>

<button
  data-slot="button"
  class={cn(
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    variants[variant],
    sizes[size],
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</button>
