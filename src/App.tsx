import { GlobalBar } from "./components/GlobalBar";
import { useStoves } from "./hooks/useStoves";

export default function App() {
  const { stoves } = useStoves();

  return (
    <main className="shell" aria-label="Cookbench">
      <GlobalBar stoves={stoves} />
    </main>
  );
}
