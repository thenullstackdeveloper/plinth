import { expect, test } from "vitest";

import config from "../.storybook/main.ts";

test("storybook never sends telemetry", () => {
  // `core` may also be a preset function; ours is a plain object.
  const core = config.core as { disableTelemetry?: boolean } | undefined;
  expect(core?.disableTelemetry).toBe(true);
});
