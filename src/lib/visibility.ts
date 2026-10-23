import type { VariableDef } from "./types";

export function isVariableVisible(
  variable: VariableDef,
  values: Record<string, unknown>,
  variables: VariableDef[],
): boolean {
  const condition = variable.visibleWhen;
  if (!condition) return true;

  const fallback = variables.find((item) => item.key === condition.key)?.default;
  const raw = values[condition.key] ?? fallback;
  const current = raw === undefined || raw === null ? "" : String(raw);

  if (condition.equals !== undefined && current !== condition.equals) {
    return false;
  }
  if (condition.notEquals !== undefined && current === condition.notEquals) {
    return false;
  }
  if (condition.in && !condition.in.includes(current)) {
    return false;
  }
  return true;
}
