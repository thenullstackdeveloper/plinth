import { useCallback, useEffect, useState } from "react";

import * as panelApi from "../adapters/panelApi";
import type { RenderView } from "../adapters/panelApi";

/** The panel flow: load what to render, and press a component. */
export function usePanel(): {
  view: RenderView | null;
  error: string | null;
  press: (componentId: string) => void;
} {
  const [view, setView] = useState<RenderView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    panelApi
      .renderState()
      .then((loaded) => {
        if (active) setView(loaded);
      })
      .catch((reason: unknown) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  const press = useCallback((componentId: string) => {
    panelApi.press(componentId).then(
      () => setError(null),
      (reason: unknown) => setError(String(reason)),
    );
  }, []);

  return { view, error, press };
}
