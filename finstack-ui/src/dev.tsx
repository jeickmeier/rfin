import React from "react";
import { createRoot } from "react-dom/client";
import { BasicRatesApp } from "./examples/BasicRatesApp";
import { InterestRateApp } from "./examples/InterestRateApp";
import bondDashboard from "../fixtures/dashboards/basic-rates/bond-and-curve.json";
import swapDashboard from "../fixtures/dashboards/basic-rates/swap-with-cashflows.json";
import calibrationDashboard from "../fixtures/dashboards/basic-rates/calibration-dashboard.json";
import "./styles.css";

const demos = {
  bond: bondDashboard,
  swap: swapDashboard,
  calibration: calibrationDashboard,
} as const;

type DemoKey = keyof typeof demos;
type AppKind = "basic" | "interest";

function getInitialStateFromLocation(): { app: AppKind; demo: DemoKey } {
  const params = new URLSearchParams(globalThis.location?.search ?? "");
  const appParam = params.get("app") === "interest" ? "interest" : "basic";
  const demoParam = (params.get("demo") ?? "bond") as DemoKey;
  const safeDemo: DemoKey = demos[demoParam] ? demoParam : "bond";
  return { app: appParam, demo: safeDemo };
}

const DevShell: React.FC = () => {
  const [{ app, demo }, setState] = React.useState<{
    app: AppKind;
    demo: DemoKey;
  }>(() => getInitialStateFromLocation());

  React.useEffect(() => {
    const params = new URLSearchParams(globalThis.location?.search ?? "");
    params.set("app", app);
    if (app === "basic") {
      params.set("demo", demo);
    } else {
      params.delete("demo");
    }
    const query = params.toString();
    const basePath = globalThis.location?.pathname ?? "/";
    const url = query ? `${basePath}?${query}` : basePath;
    globalThis.history?.replaceState?.(null, "", url);
  }, [app, demo]);

  const isBasic = app === "basic";

  return (
    <div style={{ padding: 12 }}>
      <nav style={{ marginBottom: 12, display: "flex", gap: 8 }}>
        <button
          type="button"
          onClick={() => setState((prev) => ({ ...prev, app: "basic" }))}
          style={{
            padding: "4px 8px",
            borderRadius: 4,
            border: "1px solid #ccc",
            backgroundColor: isBasic ? "#111827" : "#f9fafb",
            color: isBasic ? "#f9fafb" : "#111827",
            cursor: "pointer",
          }}
        >
          Basic Rates App
        </button>
        <button
          type="button"
          onClick={() => setState((prev) => ({ ...prev, app: "interest" }))}
          style={{
            padding: "4px 8px",
            borderRadius: 4,
            border: "1px solid #ccc",
            backgroundColor: !isBasic ? "#111827" : "#f9fafb",
            color: !isBasic ? "#f9fafb" : "#111827",
            cursor: "pointer",
          }}
        >
          Interest Rate App
        </button>
      </nav>

      {isBasic ? (
        <>
          <nav style={{ marginBottom: 12, display: "flex", gap: 8 }}>
            <button
              type="button"
              onClick={() =>
                setState((prev) => ({ ...prev, app: "basic", demo: "bond" }))
              }
              style={{
                padding: "2px 6px",
                borderRadius: 4,
                border: "1px solid #e5e7eb",
                backgroundColor: demo === "bond" ? "#e5e7eb" : "transparent",
                cursor: "pointer",
                fontSize: 12,
              }}
            >
              Bond + Discount Curve
            </button>
            <button
              type="button"
              onClick={() =>
                setState((prev) => ({ ...prev, app: "basic", demo: "swap" }))
              }
              style={{
                padding: "2px 6px",
                borderRadius: 4,
                border: "1px solid #e5e7eb",
                backgroundColor: demo === "swap" ? "#e5e7eb" : "transparent",
                cursor: "pointer",
                fontSize: 12,
              }}
            >
              Swap + Cashflows
            </button>
            <button
              type="button"
              onClick={() =>
                setState((prev) => ({
                  ...prev,
                  app: "basic",
                  demo: "calibration",
                }))
              }
              style={{
                padding: "2px 6px",
                borderRadius: 4,
                border: "1px solid #e5e7eb",
                backgroundColor:
                  demo === "calibration" ? "#e5e7eb" : "transparent",
                cursor: "pointer",
                fontSize: 12,
              }}
            >
              Calibration dashboard
            </button>
          </nav>
          <BasicRatesApp dashboard={demos[demo]} />
        </>
      ) : (
        <InterestRateApp />
      )}
    </div>
  );
};

const root = createRoot(document.getElementById("root")!);

root.render(
  <React.StrictMode>
    <DevShell />
  </React.StrictMode>,
);
