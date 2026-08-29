import mark from "./assets/cookbench-mark.svg";

export default function App() {
  return (
    <main className="shell" aria-label="Cookbench">
      <section className="bar" aria-label="Cookbench global bar">
        <div className="brand">
          <img className="mark" src={mark} alt="" />
          <span>Cookbench</span>
        </div>
        <div className="stoves" aria-label="Stoves">
          <article className="stove" data-testid="stove" aria-label="Codex cooking">
            <span className="burner burner-cooking" aria-hidden="true">
              <svg viewBox="0 0 36 36" focusable="false">
                <circle className="burner-track" cx="18" cy="18" r="13" />
                <circle className="burner-progress" cx="18" cy="18" r="13" pathLength="100" />
              </svg>
              <i />
            </span>
            <span className="stove-copy">
              <strong>Codex</strong>
              <small>Preparing workspace</small>
            </span>
          </article>
        </div>
      </section>
    </main>
  );
}
