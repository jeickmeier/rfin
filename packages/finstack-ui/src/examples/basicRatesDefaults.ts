export const defaultConfigJson = JSON.stringify({
  outputScale: 4,
  roundingModeLabel: "nearest",
});

export const defaultMarketJson = JSON.stringify({
  as_of: "2024-01-02",
  discount_curves: [
    {
      id: "USD-OIS",
      base_date: "2024-01-02",
      points: [
        { tenor_years: 0.0833, discount_factor: 0.9991 },
        { tenor_years: 0.25, discount_factor: 0.997 },
        { tenor_years: 0.5, discount_factor: 0.993 },
        { tenor_years: 1.0, discount_factor: 0.985 },
        { tenor_years: 2.0, discount_factor: 0.965 },
        { tenor_years: 5.0, discount_factor: 0.912 },
      ],
    },
  ],
});
