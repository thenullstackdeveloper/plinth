import { usePanel } from "../hooks/usePanel";

export function App() {
  const { view, error, press } = usePanel();
  const components = view?.pages[view.currentPage]?.components ?? [];

  return (
    <main>
      {components.map((component) => (
        <button key={component.id} type="button" onClick={() => press(component.id)}>
          {component.label ?? component.id}
        </button>
      ))}
      {error !== null && <p role="alert">{error}</p>}
    </main>
  );
}
