import React from "react";
import { createRoot } from "react-dom/client";
import { BasicRatesApp } from "./examples/BasicRatesApp";
import bondDashboard from "../fixtures/dashboards/basic-rates/bond-and-curve.json";
import swapDashboard from "../fixtures/dashboards/basic-rates/swap-with-cashflows.json";
import calibrationDashboard from "../fixtures/dashboards/basic-rates/calibration-dashboard.json";
import "./styles.css";

const demos = {
  bond: bondDashboard,
  swap: swapDashboard,
  calibration: calibrationDashboard,
} as const;

function pickDashboard() {
  const params = new URLSearchParams(window.location.search);
  const key = (params.get("demo") ?? "bond") as keyof typeof demos;
  return demos[key] ?? demos.bond;
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <React.StrictMode>
    <BasicRatesApp dashboard={pickDashboard()} />
  </React.StrictMode>,
);
